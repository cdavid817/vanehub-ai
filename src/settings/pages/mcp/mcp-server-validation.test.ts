import { describe, expect, it } from "vitest";
import { MCP_LIMITS, type McpTransportType } from "../../../types/mcp";
import { validateMcpServerForm, type McpServerFormValues } from "./mcp-server-validation";

const baseValues: McpServerFormValues = {
  name: "filesystem-tools",
  transportType: "stdio",
  scope: "user",
  command: "node",
  args: "server.js",
  env: "{}",
  url: "",
  headers: "{}",
  description: "",
  active: true,
};

describe("MCP server form validation", () => {
  it("builds a stdio server config from valid form values", () => {
    const result = validateMcpServerForm(baseValues);

    expect(result.success).toBe(true);
    if (result.success) {
      expect(result.config.command).toBe("node");
      expect(result.config.args).toEqual(["server.js"]);
    }
  });

  it("rejects invalid names before service submission", () => {
    const result = validateMcpServerForm({ ...baseValues, name: "Bad_Name" });

    expect(result.success).toBe(false);
    if (!result.success) {
      expect(result.errors.name).toContain("kebab-case");
    }
  });

  it("requires URL for URL transports", () => {
    const result = validateMcpServerForm({ ...baseValues, transportType: "sse", command: "", url: "" });

    expect(result.success).toBe(false);
    if (!result.success) {
      expect(result.errors.url).toContain("requires URL");
    }
  });

  it("preserves distinct legacy SSE and Streamable HTTP transports", () => {
    for (const transportType of ["sse", "streamable_http"] as const) {
      const result = validateMcpServerForm({ ...baseValues, transportType, command: "", url: "https://example.test/mcp" });
      expect(result.success).toBe(true);
      if (result.success) expect(result.config.transportType).toBe(transportType);
    }
  });

  it("rejects an unknown transport instead of interpreting it as stdio", () => {
    const result = validateMcpServerForm({
      ...baseValues,
      transportType: "http" as McpTransportType,
    });

    expect(result.success).toBe(false);
    if (!result.success) expect(result.errors.transportType).toContain("Unknown MCP transport");
  });

  it("rejects non-string args, env, and header values instead of coercing them", () => {
    for (const values of [
      { ...baseValues, args: '["ok", 1]' },
      { ...baseValues, env: '{"TOKEN": true}' },
      {
        ...baseValues,
        transportType: "streamable_http" as const,
        command: "",
        url: "https://example.test/mcp",
        headers: '{"Authorization": 42}',
      },
    ]) {
      const result = validateMcpServerForm(values);
      expect(result.success).toBe(false);
    }
  });

  it("rejects form collections above the shared entry limit", () => {
    const args = JSON.stringify(
      Array.from({ length: MCP_LIMITS.configurationCollectionEntries + 1 }, (_, index) => String(index)),
    );
    const result = validateMcpServerForm({ ...baseValues, args });

    expect(result.success).toBe(false);
    if (!result.success) expect(result.errors.args).toContain("128 entries");
  });

  it("maps JSON parse failures to the related field", () => {
    const result = validateMcpServerForm({ ...baseValues, env: "{" });

    expect(result.success).toBe(false);
    if (!result.success) {
      expect(result.errors.env).toBeTruthy();
    }
  });
});
