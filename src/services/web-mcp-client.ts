import type { McpService } from "./mcp-service";
import { parseMcpImportText } from "./mcp-import";
import {
  McpValidationError,
  validateMcpServerConfig,
  withMcpServiceValidation,
} from "./mcp-validation";
import { unsupportedRuntimeError } from "./service-error";
import type {
  McpImportExport,
  McpImportResult,
  McpScope,
  McpServerConfig,
  McpServerStatus,
  PartialMcpServerConfig,
} from "../types/mcp";
import { createWebMockOperation } from "./web-operation-client";

const defaultServers: McpServerConfig[] = [
  {
    name: "filesystem-tools",
    transportType: "stdio",
    command: "npx",
    args: ["-y", "@modelcontextprotocol/server-filesystem", "."],
    env: {},
    description: "Local filesystem tools",
    active: true,
    scope: "user",
  },
  {
    name: "remote-docs",
    transportType: "sse",
    url: "http://localhost:8000/mcp",
    headers: {},
    description: "Example legacy SSE MCP server",
    active: false,
    scope: "project",
    projectPath: "web-preview",
  },
];

const defaultStatuses: Record<string, McpServerStatus> = {
  "filesystem-tools": {
    name: "filesystem-tools",
    connectionStatus: "connected",
    tools: [
      {
        name: "read_file",
        description: "Read a file from the workspace",
        inputSchema: { type: "object" },
      },
    ],
    lastConnected: "preview",
    durationMs: 42,
  },
};

type WebMcpClientState = {
  servers?: McpServerConfig[];
  statuses?: Record<string, McpServerStatus>;
};

export function createWebMcpClient(initial: WebMcpClientState = {}): McpService {
  let servers = (initial.servers ?? defaultServers).map(cloneServer);
  const statuses = Object.fromEntries(
    Object.entries(initial.statuses ?? defaultStatuses).map(([name, status]) => [name, cloneStatus(status)]),
  );

  function addServer(config: McpServerConfig) {
    if (servers.some((server) => server.name === config.name)) {
      throw new Error(`MCP server already exists: ${config.name}`);
    }
    delete statuses[config.name];
    servers = [...servers, cloneServer(config)];
  }

  const client: McpService = {
    async listServers() {
      return servers.map(cloneServer);
    },

    async addServer(config) {
      addServer(config);
    },

    async updateServer(name: string, config: PartialMcpServerConfig) {
      const index = servers.findIndex((server) => server.name === name);
      if (index < 0) throw new Error(`MCP server not found: ${name}`);

      const updated = { ...servers[index], ...config };
      if (updated.name !== name && servers.some((server) => server.name === updated.name)) {
        throw new Error(`MCP server already exists: ${updated.name}`);
      }
      const validation = validateMcpServerConfig(updated);
      if (!validation.success) throw new McpValidationError(validation);

      servers = servers.map((server, serverIndex) => (serverIndex === index ? cloneServer(updated) : server));
      const previousStatus = statuses[name];
      delete statuses[name];
      if (updated.active && previousStatus) {
        statuses[updated.name] = { ...cloneStatus(previousStatus), name: updated.name };
      } else {
        delete statuses[updated.name];
      }
    },

    async removeServer(name: string) {
      servers = servers.filter((server) => server.name !== name);
      delete statuses[name];
    },

    async toggleServer(name: string, active: boolean) {
      await client.updateServer(name, { active });
    },

    async testConnection(name: string) {
      const server = servers.find((item) => item.name === name);
      if (!server) throw new Error(`MCP server not found: ${name}`);

      const result = {
        success: true,
        durationMs: 38,
        tools: [
          {
            name: "preview_tool",
            description: "Mock MCP tool for browser preview",
            inputSchema: { type: "object", properties: {} },
          },
        ],
      };
      if (server.active) {
        statuses[name] = {
          name,
          connectionStatus: "connected",
          tools: result.tools,
          lastConnected: "preview",
          durationMs: result.durationMs,
        };
      }
      return createWebMockOperation({
        id: `web-mcp-test-${name}-${Date.now()}`,
        kind: "mcp",
        relatedEntityId: name,
        message: `Mock MCP connection test for ${name}`,
        terminalStatus: "succeeded",
        error: null,
        result: result as unknown as Record<string, unknown>,
      });
    },

    async getServerStatus(name: string) {
      const server = servers.find((item) => item.name === name);
      if (!server?.active) {
        return { name, connectionStatus: server ? "disabled" : "disconnected", tools: [] };
      }
      return statuses[name] ? cloneStatus(statuses[name]) : { name, connectionStatus: "disconnected", tools: [] };
    },

    async callTool() {
      throw unsupportedRuntimeError("MCP tool calling is reserved for a later VaneHub release.");
    },

    async importServers(input: string, scope: McpScope): Promise<McpImportResult> {
      const parsed = parseMcpImportText(input);
      const imported: string[] = [];
      const skipped: string[] = [];
      for (const [name, entry] of Object.entries(parsed.data.mcpServers)) {
        if (servers.some((server) => server.name === name)) {
          skipped.push(name);
          continue;
        }
        addServer({
          name,
          transportType: entry.command ? "stdio" : entry.type === "sse" ? "sse" : "streamable_http",
          command: entry.command,
          args: entry.args,
          env: entry.env,
          url: entry.url,
          headers: entry.headers,
          active: true,
          scope,
          projectPath: scope === "project" ? "web-preview" : undefined,
        });
        imported.push(name);
      }
      return { imported, skipped, failures: parsed.failures };
    },

    async exportServers(names: string[]): Promise<McpImportExport> {
      const mcpServers: McpImportExport["mcpServers"] = {};
      for (const server of servers.filter((item) => names.includes(item.name))) {
        mcpServers[server.name] =
          server.transportType === "stdio"
            ? {
                command: server.command ?? undefined,
                args: server.args ?? undefined,
                env: server.env ?? undefined,
              }
            : {
                type: server.transportType === "sse" ? "sse" : "http",
                url: server.url ?? undefined,
                headers: server.headers ?? undefined,
              };
      }
      return { mcpServers };
    },
  };

  return withMcpServiceValidation(client);
}

function cloneServer(server: McpServerConfig): McpServerConfig {
  return {
    ...server,
    args: server.args ? [...server.args] : server.args,
    env: server.env ? { ...server.env } : server.env,
    headers: server.headers ? { ...server.headers } : server.headers,
  };
}

function cloneStatus(status: McpServerStatus): McpServerStatus {
  return {
    ...status,
    tools: status.tools.map((tool) => ({
      ...tool,
      inputSchema: tool.inputSchema ? { ...tool.inputSchema } : tool.inputSchema,
    })),
  };
}

export const webMcpClient = createWebMcpClient();
