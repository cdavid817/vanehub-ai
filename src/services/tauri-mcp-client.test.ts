import { readFileSync, readdirSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";
import { beforeEach, describe, expect, it, vi } from "vitest";

const { invokeMock } = vi.hoisted(() => ({ invokeMock: vi.fn() }));

vi.mock("@tauri-apps/api/core", () => ({ invoke: invokeMock }));

import { tauriMcpClient } from "./tauri-mcp-client";
import { withMcpServiceValidation } from "./mcp-validation";
import { MCP_LIMITS, type McpServerConfig } from "../types/mcp";

const currentDir = dirname(fileURLToPath(import.meta.url));

describe("Tauri MCP adapter", () => {
  beforeEach(() => invokeMock.mockReset());

  it("maps safe error codes on status and operation results", async () => {
    invokeMock.mockResolvedValueOnce({
      name: "fixture-tools",
      connectionStatus: "error",
      tools: [],
      error: "The MCP operation timed out.",
      errorCode: "timeout",
      durationMs: 17,
    });

    await expect(tauriMcpClient.getServerStatus("fixture-tools")).resolves.toMatchObject({
      errorCode: "timeout",
    });
    expect(invokeMock).toHaveBeenLastCalledWith("get_mcp_server_status", {
      name: "fixture-tools",
    });

    invokeMock.mockResolvedValueOnce({
      id: "op-1",
      kind: "mcp",
      status: "failed",
      logs: [],
      result: {
        success: false,
        operationId: "op-1",
        tools: [],
        error: "The MCP resource limit was exceeded.",
        errorCode: "limit_exceeded",
      },
      error: null,
      createdAt: "100",
      updatedAt: "101",
    });

    await expect(tauriMcpClient.testConnection("fixture-tools")).resolves.toMatchObject({
      result: { errorCode: "limit_exceeded" },
    });
    expect(invokeMock).toHaveBeenLastCalledWith("test_mcp_connection", {
      name: "fixture-tools",
    });
  });

  it("does not expose an unknown native error classification as a safe code", async () => {
    invokeMock.mockResolvedValueOnce({
      name: "fixture-tools",
      connectionStatus: "error",
      tools: [],
      errorCode: "future_private_code",
    });
    await expect(tauriMcpClient.getServerStatus("fixture-tools")).resolves.toMatchObject({
      errorCode: null,
    });

    invokeMock.mockResolvedValueOnce({
      id: "op-1",
      kind: "mcp",
      status: "failed",
      logs: [],
      result: { success: false, tools: [], errorCode: "future_private_code" },
      createdAt: "100",
      updatedAt: "101",
    });
    await expect(tauriMcpClient.testConnection("fixture-tools")).resolves.toMatchObject({
      result: { errorCode: null },
    });
  });

  it("passes explicit compatible URL transport markers through the Tauri boundary", async () => {
    const data = {
      mcpServers: {
        legacy: { type: "sse" as const, url: "https://fixture.example/events" },
        modern: { type: "http" as const, url: "https://fixture.example/mcp" },
      },
    };
    invokeMock.mockResolvedValue({ imported: ["legacy", "modern"], skipped: [], failures: [] });

    await tauriMcpClient.importServers(JSON.stringify(data), "user");

    expect(invokeMock).toHaveBeenCalledWith("import_mcp_servers", { data, scope: "user" });
  });

  it("merges frontend validation and native storage feedback per import entry", async () => {
    invokeMock.mockResolvedValue({
      imported: [],
      skipped: [],
      failures: [
        {
          name: "valid-server",
          stage: "storage",
          errorCode: null,
          message: "The MCP server could not be saved.",
        },
      ],
    });
    const input = JSON.stringify({
      mcpServers: {
        Bad_Name: { command: "node" },
        "valid-server": { command: "node" },
      },
    });

    await expect(tauriMcpClient.importServers(input, "user")).resolves.toMatchObject({
      failures: [
        { name: "Bad_Name", stage: "validation", errorCode: "validation" },
        { name: "valid-server", stage: "storage", errorCode: null },
      ],
    });
    expect(invokeMock).toHaveBeenCalledWith("import_mcp_servers", {
      data: { mcpServers: { "valid-server": { command: "node" } } },
      scope: "user",
    });
  });

  it("keeps MCP invoke calls confined to this adapter", () => {
    const files = [
      join(currentDir, "mcp-service.ts"),
      join(currentDir, "runtime-mcp-client.ts"),
      join(currentDir, "web-mcp-client.ts"),
      join(currentDir, "..", "settings", "pages", "mcp-page.tsx"),
      ...readdirSync(join(currentDir, "..", "settings", "pages", "mcp"))
        .filter((name) => name.endsWith(".ts") || name.endsWith(".tsx"))
        .map((name) => join(currentDir, "..", "settings", "pages", "mcp", name)),
    ];

    for (const file of files) {
      const source = readFileSync(file, "utf8");
      expect(source, file).not.toContain("@tauri-apps/api");
      expect(source, file).not.toContain("invoke(");
    }
  });

  it("applies shared validation before the desktop adapter invokes native code", async () => {
    const validated = withMcpServiceValidation(tauriMcpClient);
    const config: McpServerConfig = {
      name: "fixture-tools",
      transportType: "stdio",
      command: "node",
      args: [],
      env: {},
      active: true,
      scope: "user",
    };

    await expect(
      validated.addServer({ ...config, transportType: "future" } as unknown as McpServerConfig),
    ).rejects.toMatchObject({ errorCode: "validation", field: "transportType" });
    await expect(
      validated.addServer({
        ...config,
        args: Array.from({ length: MCP_LIMITS.configurationCollectionEntries + 1 }, (_, index) => `${index}`),
      }),
    ).rejects.toMatchObject({ errorCode: "limit_exceeded", field: "args" });
    expect(invokeMock).not.toHaveBeenCalled();
  });
});
