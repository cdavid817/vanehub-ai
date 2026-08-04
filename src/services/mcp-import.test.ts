import { describe, expect, it, vi } from "vitest";
import { parseMcpImportText, previewMcpImportNames } from "./mcp-import";
import { MCP_LIMITS } from "../types/mcp";

describe("MCP import parsing", () => {
  it("checks UTF-8 bytes before JSON.parse and accepts the exact boundary", () => {
    const document = '{"mcpServers":{}}';
    const exact = `${" ".repeat(MCP_LIMITS.importDocumentBytes - document.length)}${document}`;
    expect(new TextEncoder().encode(exact).byteLength).toBe(MCP_LIMITS.importDocumentBytes);
    expect(parseMcpImportText(exact).data).toEqual({ mcpServers: {} });

    const parseSpy = vi.spyOn(JSON, "parse");
    expect(() => parseMcpImportText(` ${exact}`)).toThrowError(
      expect.objectContaining({ errorCode: "limit_exceeded", field: "import" }),
    );
    expect(parseSpy).not.toHaveBeenCalled();
    parseSpy.mockRestore();
  });

  it("accepts 128 entries and rejects 129 after parsing", () => {
    const entries = Object.fromEntries(
      Array.from({ length: MCP_LIMITS.importServerEntries }, (_, index) => [
        `server-${index}`,
        { command: "node" },
      ]),
    );
    expect(Object.keys(parseMcpImportText(JSON.stringify({ mcpServers: entries })).data.mcpServers)).toHaveLength(
      MCP_LIMITS.importServerEntries,
    );

    entries.extra = { command: "node" };
    expect(() => parseMcpImportText(JSON.stringify({ mcpServers: entries }))).toThrowError(
      expect.objectContaining({ errorCode: "limit_exceeded" }),
    );
  });

  it("normalizes explicit and historical transport markers", () => {
    const parsed = parseMcpImportText(
      JSON.stringify({
        mcpServers: {
          command: { type: "sse", command: "node", args: ["server.js"] },
          legacy: { type: "sse", url: "https://example.test/events" },
          http: { type: "http", url: "https://example.test/http" },
          streamable: { type: "streamable_http", url: "https://example.test/streamable" },
          historical: { url: "https://example.test/historical" },
        },
      }),
    );

    expect(parsed.failures).toEqual([]);
    expect(parsed.data.mcpServers.command).toEqual({ command: "node", args: ["server.js"], env: undefined });
    expect(parsed.data.mcpServers.legacy?.type).toBe("sse");
    for (const name of ["http", "streamable", "historical"]) {
      expect(parsed.data.mcpServers[name]?.type).toBe("http");
    }
  });

  it("returns concise per-entry validation failures while retaining valid entries", () => {
    const parsed = parseMcpImportText(
      JSON.stringify({
        mcpServers: {
          valid: { command: "node", env: { MODE: "test" } },
          Bad_Name: { command: "node" },
          unknown: { type: "future", url: "https://example.test/mcp" },
          numeric: { command: "node", env: { TOKEN: 42 } },
        },
      }),
    );

    expect(Object.keys(parsed.data.mcpServers)).toEqual(["valid"]);
    expect(parsed.failures).toHaveLength(3);
    expect(parsed.failures).toEqual(
      expect.arrayContaining([
        expect.objectContaining({ name: "Bad_Name", stage: "validation", errorCode: "validation" }),
        expect.objectContaining({ name: "unknown", stage: "validation", errorCode: "validation" }),
        expect.objectContaining({ name: "numeric", stage: "validation", errorCode: "validation" }),
      ]),
    );
  });

  it("uses the bounded parser for preview names", () => {
    expect(previewMcpImportNames('{"mcpServers":{"valid":{"command":"node"}}}')).toEqual(["valid"]);
    expect(previewMcpImportNames("not-json")).toEqual([]);
    expect(previewMcpImportNames("x".repeat(MCP_LIMITS.importDocumentBytes + 1))).toEqual([]);
  });
});
