import { describe, expect, it } from "vitest";
import { MCP_LIMITS, type McpServerConfig, type McpServerStatus } from "../types/mcp";
import { createWebMcpClient } from "./web-mcp-client";

const stdioServer: McpServerConfig = {
  name: "local-tools",
  transportType: "stdio",
  command: "node",
  args: ["server.js"],
  env: {},
  active: true,
  scope: "user",
};

const connectedStatus: McpServerStatus = {
  name: stdioServer.name,
  connectionStatus: "connected",
  tools: [{ name: "read", inputSchema: { type: "object" } }],
  lastConnected: "preview",
};

describe("Web MCP client", () => {
  it("keeps legacy SSE distinct from Streamable HTTP across import and export", async () => {
    const client = createWebMcpClient({ servers: [], statuses: {} });
    const result = await client.importServers(
      JSON.stringify({
        mcpServers: {
          command: { command: "node", args: ["server.js"] },
          legacy: { type: "sse", url: "https://example.test/events" },
          http: { type: "http", url: "https://example.test/mcp" },
          streamable: { type: "streamable_http", url: "https://example.test/streamable" },
          historical: { url: "https://example.test/historical" },
        },
      }),
      "project",
    );

    expect(result).toEqual({
      imported: ["command", "legacy", "http", "streamable", "historical"],
      skipped: [],
      failures: [],
    });
    expect(
      Object.fromEntries((await client.listServers()).map((server) => [server.name, server.transportType])),
    ).toEqual({
      command: "stdio",
      legacy: "sse",
      http: "streamable_http",
      streamable: "streamable_http",
      historical: "streamable_http",
    });

    const exported = await client.exportServers(["legacy", "http", "streamable", "historical"]);
    expect(exported.mcpServers.legacy?.type).toBe("sse");
    for (const name of ["http", "streamable", "historical"]) {
      expect(exported.mcpServers[name]?.type).toBe("http");
    }
  });

  it("returns shared validation and limit codes before changing mock state", async () => {
    const client = createWebMcpClient({ servers: [], statuses: {} });

    await expect(
      client.addServer({ ...stdioServer, transportType: "future" } as unknown as McpServerConfig),
    ).rejects.toMatchObject({ errorCode: "validation", field: "transportType" });
    await expect(
      client.addServer({
        ...stdioServer,
        args: Array.from({ length: MCP_LIMITS.configurationCollectionEntries + 1 }, (_, index) => `${index}`),
      }),
    ).rejects.toMatchObject({ errorCode: "limit_exceeded", field: "args" });
    await client.addServer(stdioServer);
    await expect(client.updateServer(stdioServer.name, { transportType: "sse" })).rejects.toMatchObject({
      errorCode: "validation",
      field: "url",
    });
    await expect(
      client.callTool("local-tools", "read", nestedObject(MCP_LIMITS.jsonDepth + 1)),
    ).rejects.toMatchObject({ errorCode: "limit_exceeded", field: "arguments" });
    await expect(client.importServers("x".repeat(MCP_LIMITS.importDocumentBytes + 1), "user")).rejects.toMatchObject({
      errorCode: "limit_exceeded",
      field: "import",
    });

    expect(await client.listServers()).toEqual([stdioServer]);
  });

  it("migrates connected status when a server is renamed", async () => {
    const client = createWebMcpClient({
      servers: [stdioServer],
      statuses: { [stdioServer.name]: connectedStatus },
    });

    await client.updateServer(stdioServer.name, { name: "renamed-tools" });

    expect(await client.getServerStatus(stdioServer.name)).toMatchObject({
      name: stdioServer.name,
      connectionStatus: "disconnected",
      tools: [],
    });
    expect(await client.getServerStatus("renamed-tools")).toMatchObject({
      name: "renamed-tools",
      connectionStatus: "connected",
      tools: [{ name: "read" }],
    });
  });

  it("removes cached status when a server is disabled and does not restore it on enable", async () => {
    const client = createWebMcpClient({
      servers: [stdioServer],
      statuses: { [stdioServer.name]: connectedStatus },
    });

    await client.toggleServer(stdioServer.name, false);
    expect(await client.getServerStatus(stdioServer.name)).toEqual({
      name: stdioServer.name,
      connectionStatus: "disabled",
      tools: [],
    });

    await client.toggleServer(stdioServer.name, true);
    expect(await client.getServerStatus(stdioServer.name)).toEqual({
      name: stdioServer.name,
      connectionStatus: "disconnected",
      tools: [],
    });
  });

  it("does not retain status through remove, re-add, or import", async () => {
    const client = createWebMcpClient({
      servers: [stdioServer],
      statuses: { [stdioServer.name]: connectedStatus },
    });

    await client.removeServer(stdioServer.name);
    await client.addServer(stdioServer);
    expect(await client.getServerStatus(stdioServer.name)).toMatchObject({
      connectionStatus: "disconnected",
      tools: [],
    });

    await client.removeServer(stdioServer.name);
    await client.importServers(
      JSON.stringify({ mcpServers: { [stdioServer.name]: { command: "node", args: ["server.js"] } } }),
      "user",
    );
    expect(await client.getServerStatus(stdioServer.name)).toMatchObject({
      connectionStatus: "disconnected",
      tools: [],
    });
  });

  it("returns copies so callers cannot mutate mock configuration or status", async () => {
    const client = createWebMcpClient({
      servers: [stdioServer],
      statuses: { [stdioServer.name]: connectedStatus },
    });

    const listed = await client.listServers();
    listed[0].args?.push("mutated");
    const status = await client.getServerStatus(stdioServer.name);
    status.tools.length = 0;

    expect((await client.listServers())[0].args).toEqual(["server.js"]);
    expect((await client.getServerStatus(stdioServer.name)).tools).toHaveLength(1);
  });
});

function nestedObject(depth: number): Record<string, unknown> {
  let value: unknown = null;
  for (let index = 1; index < depth; index += 1) value = { nested: value };
  return value as Record<string, unknown>;
}
