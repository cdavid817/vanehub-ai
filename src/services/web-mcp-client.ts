import type { McpService } from "./mcp-service";
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

let mockServers: McpServerConfig[] = [
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
    description: "Example URL MCP server",
    active: false,
    scope: "project",
    projectPath: "web-preview",
  },
];

const mockStatuses: Record<string, McpServerStatus> = {
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
  "remote-docs": {
    name: "remote-docs",
    connectionStatus: "disabled",
    tools: [],
    error: null,
  },
};

export const webMcpClient: McpService = {
  async listServers() {
    return mockServers;
  },

  async addServer(config) {
    if (mockServers.some((server) => server.name === config.name)) {
      throw new Error(`MCP server already exists: ${config.name}`);
    }
    mockServers = [...mockServers, config];
  },

  async updateServer(name: string, config: PartialMcpServerConfig) {
    mockServers = mockServers.map((server) => (server.name === name ? { ...server, ...config } : server));
  },

  async removeServer(name: string) {
    mockServers = mockServers.filter((server) => server.name !== name);
  },

  async toggleServer(name: string, active: boolean) {
    await this.updateServer(name, { active });
  },

  async testConnection(name: string) {
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
    mockStatuses[name] = {
      name,
      connectionStatus: "connected",
      tools: result.tools,
      lastConnected: "preview",
      durationMs: result.durationMs,
    };
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
    const server = mockServers.find((item) => item.name === name);
    return (
      mockStatuses[name] ?? {
        name,
        connectionStatus: server?.active === false ? "disabled" : "disconnected",
        tools: [],
      }
    );
  },

  async callTool() {
    throw unsupportedRuntimeError("MCP tool calling is reserved for a later VaneHub release.");
  },

  async importServers(data: McpImportExport, scope: McpScope): Promise<McpImportResult> {
    const imported: string[] = [];
    const skipped: string[] = [];
    for (const [name, entry] of Object.entries(data.mcpServers)) {
      if (mockServers.some((server) => server.name === name)) {
        skipped.push(name);
        continue;
      }
      mockServers = [
        ...mockServers,
        {
          name,
          transportType: entry.command ? "stdio" : "sse",
          command: entry.command,
          args: entry.args,
          env: entry.env,
          url: entry.url,
          headers: entry.headers,
          active: true,
          scope,
          projectPath: scope === "project" ? "web-preview" : undefined,
        },
      ];
      imported.push(name);
    }
    return { imported, skipped };
  },

  async exportServers(names: string[]): Promise<McpImportExport> {
    const mcpServers: McpImportExport["mcpServers"] = {};
    for (const server of mockServers.filter((item) => names.includes(item.name))) {
      mcpServers[server.name] =
        server.transportType === "stdio"
          ? {
              command: server.command ?? undefined,
              args: server.args ?? undefined,
              env: server.env ?? undefined,
            }
          : {
              url: server.url ?? undefined,
              headers: server.headers ?? undefined,
            };
    }
    return { mcpServers };
  },
};
