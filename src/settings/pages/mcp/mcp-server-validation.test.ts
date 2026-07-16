import { describe, expect, it } from "vitest";
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

  it("maps JSON parse failures to the related field", () => {
    const result = validateMcpServerForm({ ...baseValues, env: "{" });

    expect(result.success).toBe(false);
    if (!result.success) {
      expect(result.errors.env).toBeTruthy();
    }
  });
});
