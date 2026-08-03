import { describe, expect, it, vi } from "vitest";
import type { McpService } from "./mcp-service";
import {
  mcpTransportLabel,
  validateMcpServerConfig,
  validateMcpToolArguments,
  withMcpServiceValidation,
} from "./mcp-validation";
import { MCP_LIMITS, type McpServerConfig } from "../types/mcp";

const encoder = new TextEncoder();

const baseConfig: McpServerConfig = {
  name: "filesystem-tools",
  transportType: "stdio",
  command: "node",
  args: [],
  env: {},
  url: null,
  headers: null,
  description: null,
  active: true,
  scope: "user",
};

describe("MCP service validation", () => {
  it("uses distinct truthful labels and rejects unknown transports", () => {
    expect(mcpTransportLabel("sse")).toBe("legacy SSE");
    expect(mcpTransportLabel("streamable_http")).toBe("Streamable HTTP");

    const result = validateMcpServerConfig({ ...baseConfig, transportType: "http" });
    expect(result).toMatchObject({ success: false, errorCode: "validation", field: "transportType" });
  });

  it("accepts collection boundaries and rejects limit plus one", () => {
    const exact = Array.from({ length: MCP_LIMITS.configurationCollectionEntries }, (_, index) => String(index));
    const exactRecord = Object.fromEntries(exact.map((key) => [key, "value"]));
    expect(validateMcpServerConfig({ ...baseConfig, args: exact })).toEqual({ success: true });
    expect(validateMcpServerConfig({ ...baseConfig, args: [...exact, "extra"] })).toMatchObject({
      success: false,
      errorCode: "limit_exceeded",
      field: "args",
    });
    expect(validateMcpServerConfig({ ...baseConfig, env: exactRecord })).toEqual({ success: true });
    expect(validateMcpServerConfig({ ...baseConfig, env: { ...exactRecord, extra: "value" } })).toMatchObject({
      success: false,
      errorCode: "limit_exceeded",
      field: "env",
    });

    const urlConfig = {
      ...baseConfig,
      transportType: "streamable_http" as const,
      command: null,
      args: null,
      env: null,
      url: "https://example.test/mcp",
      headers: exactRecord,
    };
    expect(validateMcpServerConfig(urlConfig)).toEqual({ success: true });
    expect(validateMcpServerConfig({ ...urlConfig, headers: { ...exactRecord, extra: "value" } })).toMatchObject({
      success: false,
      errorCode: "limit_exceeded",
      field: "headers",
    });
  });

  it("rejects non-string args, environment values, and header values", () => {
    expect(validateMcpServerConfig({ ...baseConfig, args: ["ok", 1] })).toMatchObject({
      success: false,
      errorCode: "validation",
      field: "args",
    });
    expect(validateMcpServerConfig({ ...baseConfig, env: { TOKEN: true } })).toMatchObject({
      success: false,
      errorCode: "validation",
      field: "env",
    });
    expect(
      validateMcpServerConfig({
        ...baseConfig,
        transportType: "sse",
        command: null,
        args: null,
        env: null,
        url: "https://example.test/events",
        headers: { Authorization: 42 },
      }),
    ).toMatchObject({ success: false, errorCode: "validation", field: "headers" });
  });

  it("accepts the serialized configuration byte boundary and rejects limit plus one", () => {
    const emptyCommand = { ...baseConfig, command: "" };
    const baseSize = transportConfigurationBytes(emptyCommand);
    const exact = {
      ...baseConfig,
      command: "x".repeat(MCP_LIMITS.configurationSerializedBytes - baseSize),
    };

    expect(transportConfigurationBytes(exact)).toBe(MCP_LIMITS.configurationSerializedBytes);
    expect(validateMcpServerConfig(exact)).toEqual({ success: true });
    expect(validateMcpServerConfig({ ...exact, command: `${exact.command}x` })).toMatchObject({
      success: false,
      errorCode: "limit_exceeded",
      field: "form",
    });
  });

  it("accepts tool argument depth and byte boundaries and rejects limit plus one", () => {
    expect(validateMcpToolArguments(nestedObject(MCP_LIMITS.jsonDepth))).toEqual({ success: true });
    expect(validateMcpToolArguments(nestedObject(MCP_LIMITS.jsonDepth + 1))).toMatchObject({
      success: false,
      errorCode: "limit_exceeded",
      field: "arguments",
    });

    const base = { value: "" };
    const baseSize = encoder.encode(JSON.stringify(base)).byteLength;
    const exact = { value: "x".repeat(MCP_LIMITS.toolArgumentsBytes - baseSize) };
    expect(encoder.encode(JSON.stringify(exact)).byteLength).toBe(MCP_LIMITS.toolArgumentsBytes);
    expect(validateMcpToolArguments(exact)).toEqual({ success: true });
    expect(validateMcpToolArguments({ value: `${exact.value}x` })).toMatchObject({
      success: false,
      errorCode: "limit_exceeded",
    });
  });

  it("validates mutations before calling the selected runtime adapter", async () => {
    const addServer = vi.fn(async () => undefined);
    const updateServer = vi.fn(async () => undefined);
    const callTool = vi.fn(async () => ({ content: "ok", isError: false }));
    const validated = withMcpServiceValidation(mockService({ addServer, updateServer, callTool }));

    await expect(
      validated.addServer({ ...baseConfig, transportType: "unknown" } as unknown as McpServerConfig),
    ).rejects.toMatchObject({ errorCode: "validation", field: "transportType" });
    await expect(validated.updateServer(baseConfig.name, { env: { TOKEN: 42 } as unknown as Record<string, string> }))
      .rejects.toMatchObject({ errorCode: "validation", field: "env" });
    await expect(validated.callTool(baseConfig.name, "read", nestedObject(MCP_LIMITS.jsonDepth + 1)))
      .rejects.toMatchObject({ errorCode: "limit_exceeded", field: "arguments" });

    expect(addServer).not.toHaveBeenCalled();
    expect(updateServer).not.toHaveBeenCalled();
    expect(callTool).not.toHaveBeenCalled();
  });
});

function transportConfigurationBytes(config: McpServerConfig): number {
  return encoder.encode(
    JSON.stringify({
      transportType: config.transportType ?? null,
      command: config.command ?? null,
      args: config.args ?? null,
      env: config.env ?? null,
      url: config.url ?? null,
      headers: config.headers ?? null,
    }),
  ).byteLength;
}

function nestedObject(depth: number): Record<string, unknown> {
  let value: unknown = null;
  for (let index = 1; index < depth; index += 1) value = { nested: value };
  return value as Record<string, unknown>;
}

function mockService(overrides: Partial<McpService>): McpService {
  return {
    listServers: async () => [],
    addServer: async () => undefined,
    updateServer: async () => undefined,
    removeServer: async () => undefined,
    toggleServer: async () => undefined,
    testConnection: async () => {
      throw new Error("unused");
    },
    getServerStatus: async () => ({ name: "unused", connectionStatus: "disconnected", tools: [] }),
    callTool: async () => ({ content: "", isError: false }),
    importServers: async () => ({ imported: [], skipped: [], failures: [] }),
    exportServers: async () => ({ mcpServers: {} }),
    ...overrides,
  };
}
