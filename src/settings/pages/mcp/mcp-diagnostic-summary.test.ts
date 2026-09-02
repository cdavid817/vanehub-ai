import type { TFunction } from "i18next";
import { beforeAll, describe, expect, it } from "vitest";
import { activateAppLanguage, i18n } from "../../../i18n";
import { formatDiagnosticSummary } from "../../../ui/diagnostics/diagnostic-field";
import type { McpServerConfig, McpServerStatus } from "../../../types/mcp";
import { buildMcpDiagnosticFields } from "./mcp-diagnostic-summary";

let t: TFunction;
beforeAll(async () => {
  await activateAppLanguage("en");
  t = i18n.getFixedT("en");
});

function stdioServer(overrides: Partial<McpServerConfig> = {}): McpServerConfig {
  return {
    name: "filesystem-tools",
    transportType: "stdio",
    command: "npx",
    args: ["-y", "@modelcontextprotocol/server-filesystem"],
    env: null,
    active: true,
    scope: "user",
    ...overrides,
  };
}

function httpServer(overrides: Partial<McpServerConfig> = {}): McpServerConfig {
  return {
    name: "remote-tools",
    transportType: "streamable_http",
    url: "https://mcp.example.test/tools",
    headers: null,
    active: true,
    scope: "project",
    ...overrides,
  };
}

function status(overrides: Partial<McpServerStatus> = {}): McpServerStatus {
  return {
    name: "filesystem-tools",
    connectionStatus: "connected",
    tools: [],
    lastConnected: "1700000000",
    errorCode: null,
    durationMs: 42,
    ...overrides,
  };
}

// Realistic-looking secret shapes, the same defensive-fixture role SSH's own SECRET_HOST/etc. and
// IM's own SECRET_VALUE play: if the env/headers exclusion were ever accidentally weakened to a
// "keys but not values" partial reveal, or a naive dump of the raw map, these would be the first
// thing that leaked.
const SECRET_ENV_VALUE = "sk-live-do-not-leak-1234567890";
const SECRET_HEADER_VALUE = "Bearer do-not-leak-header-token";

describe("buildMcpDiagnosticFields (redaction)", () => {
  it("never includes env values or key names, even planted next to a safe config field", () => {
    const fields = buildMcpDiagnosticFields(
      stdioServer({ env: { API_KEY: SECRET_ENV_VALUE, MODE: "production" } }),
      status(),
      t,
    );
    const summary = formatDiagnosticSummary(fields, "unavailable");
    expect(summary).not.toContain(SECRET_ENV_VALUE);
    expect(summary).not.toContain("API_KEY");
    expect(summary).not.toContain("MODE");
    expect(summary).not.toContain("production");
  });

  it("never includes header values or key names for an HTTP-transport server", () => {
    const fields = buildMcpDiagnosticFields(
      httpServer({ headers: { Authorization: SECRET_HEADER_VALUE, "X-Trace-Id": "trace-abc" } }),
      status({ name: "remote-tools" }),
      t,
    );
    const summary = formatDiagnosticSummary(fields, "unavailable");
    expect(summary).not.toContain(SECRET_HEADER_VALUE);
    expect(summary).not.toContain("Authorization");
    expect(summary).not.toContain("X-Trace-Id");
    expect(summary).not.toContain("trace-abc");
  });

  it("reports hasEnv/hasHeaders as boolean flags, never the map's keys or values", () => {
    const withEnv = buildMcpDiagnosticFields(stdioServer({ env: { TOKEN: SECRET_ENV_VALUE } }), status(), t);
    const byLabelWithEnv = new Map(withEnv.map((field) => [field.label, field.value]));
    expect(byLabelWithEnv.get(t("mcp.diagnostics.field.hasEnv"))).toBe("true");
    expect(formatDiagnosticSummary(withEnv, "unavailable")).not.toContain(SECRET_ENV_VALUE);

    const withoutEnv = buildMcpDiagnosticFields(stdioServer({ env: null }), status(), t);
    const byLabelWithoutEnv = new Map(withoutEnv.map((field) => [field.label, field.value]));
    expect(byLabelWithoutEnv.get(t("mcp.diagnostics.field.hasEnv"))).toBe("false");

    const emptyEnv = buildMcpDiagnosticFields(stdioServer({ env: {} }), status(), t);
    const byLabelEmptyEnv = new Map(emptyEnv.map((field) => [field.label, field.value]));
    expect(byLabelEmptyEnv.get(t("mcp.diagnostics.field.hasEnv"))).toBe("false");
  });

  it("marks hasEnv/hasHeaders unavailable when the server's transport doesn't apply to them", () => {
    const stdio = buildMcpDiagnosticFields(stdioServer({ env: { A: "b" } }), status(), t);
    const byLabelStdio = new Map(stdio.map((field) => [field.label, field.value]));
    expect(byLabelStdio.get(t("mcp.diagnostics.field.hasEnv"))).toBe("true");
    expect(byLabelStdio.get(t("mcp.diagnostics.field.hasHeaders"))).toBeNull();

    const http = buildMcpDiagnosticFields(httpServer({ headers: { A: "b" } }), status({ name: "remote-tools" }), t);
    const byLabelHttp = new Map(http.map((field) => [field.label, field.value]));
    expect(byLabelHttp.get(t("mcp.diagnostics.field.hasHeaders"))).toBe("true");
    expect(byLabelHttp.get(t("mcp.diagnostics.field.hasEnv"))).toBeNull();
  });

  it("never carries anything beyond the bounded fields this snapshot type can hold", () => {
    const fields = buildMcpDiagnosticFields(stdioServer({ env: { A: SECRET_ENV_VALUE } }), status(), t);
    expect(fields.every((field) => typeof field.label === "string" && (field.value === null || typeof field.value === "string"))).toBe(true);
  });
});

describe("buildMcpDiagnosticFields", () => {
  it("reports raw identity, transport, and config fields for a stdio server", () => {
    const fields = buildMcpDiagnosticFields(
      stdioServer({ name: "filesystem-tools", command: "npx", args: ["-y", "server"], scope: "user" }),
      status({ name: "filesystem-tools", connectionStatus: "connected", durationMs: 17, errorCode: null, lastConnected: "1700000000" }),
      t,
    );
    const byLabel = new Map(fields.map((field) => [field.label, field.value]));
    expect(byLabel.get(t("mcp.form.name"))).toBe("filesystem-tools");
    expect(byLabel.get(t("mcp.form.transport"))).toBe("stdio");
    expect(byLabel.get(t("mcp.form.scope"))).toBe("user");
    expect(byLabel.get(t("mcp.form.enabled"))).toBe("true");
    expect(byLabel.get(t("mcp.form.command"))).toBe("npx");
    expect(byLabel.get(t("mcp.form.args"))).toBe("-y server");
    expect(byLabel.get(t("mcp.diagnostics.field.connectionStatus"))).toBe("connected");
    expect(byLabel.get(t("mcp.diagnostics.field.errorCode"))).toBeNull();
    expect(byLabel.get(t("mcp.diagnostics.field.durationMs"))).toBe("17");
    expect(byLabel.get(t("mcp.diagnostics.field.lastConnected"))).toBe("1700000000");
    expect(byLabel.get(t("mcp.form.url"))).toBeNull();
  });

  it("reports url and a remediation error code, and marks command/args unavailable, for an HTTP-transport server", () => {
    const fields = buildMcpDiagnosticFields(
      httpServer({ name: "remote-tools", transportType: "sse", url: "https://mcp.example.test/tools" }),
      status({ name: "remote-tools", connectionStatus: "error", errorCode: "upstream_http" }),
      t,
    );
    const byLabel = new Map(fields.map((field) => [field.label, field.value]));
    expect(byLabel.get(t("mcp.form.transport"))).toBe("sse");
    expect(byLabel.get(t("mcp.form.url"))).toBe("https://mcp.example.test/tools");
    expect(byLabel.get(t("mcp.diagnostics.field.connectionStatus"))).toBe("error");
    expect(byLabel.get(t("mcp.diagnostics.field.errorCode"))).toBe("upstream_http");
    expect(byLabel.get(t("mcp.form.command"))).toBeNull();
    expect(byLabel.get(t("mcp.form.args"))).toBeNull();
  });

  it("collapses an empty args list to unavailable rather than an empty line", () => {
    const fields = buildMcpDiagnosticFields(stdioServer({ args: [] }), status(), t);
    const byLabel = new Map(fields.map((field) => [field.label, field.value]));
    expect(byLabel.get(t("mcp.form.args"))).toBeNull();
  });

  it("marks connection-status fields unavailable before the first status load, rather than guessing", () => {
    const fields = buildMcpDiagnosticFields(stdioServer(), undefined, t);
    const byLabel = new Map(fields.map((field) => [field.label, field.value]));
    expect(byLabel.get(t("mcp.diagnostics.field.connectionStatus"))).toBeNull();
    expect(byLabel.get(t("mcp.diagnostics.field.errorCode"))).toBeNull();
    expect(byLabel.get(t("mcp.diagnostics.field.durationMs"))).toBeNull();
    expect(byLabel.get(t("mcp.diagnostics.field.lastConnected"))).toBeNull();
  });

  it('reports a zero duration as "0", not unavailable', () => {
    const fields = buildMcpDiagnosticFields(stdioServer(), status({ durationMs: 0 }), t);
    const byLabel = new Map(fields.map((field) => [field.label, field.value]));
    expect(byLabel.get(t("mcp.diagnostics.field.durationMs"))).toBe("0");
  });

  it("reports active as a raw boolean flag whether the server is enabled or disabled", () => {
    const enabled = buildMcpDiagnosticFields(stdioServer({ active: true }), status(), t);
    expect(new Map(enabled.map((field) => [field.label, field.value])).get(t("mcp.form.enabled"))).toBe("true");

    const disabled = buildMcpDiagnosticFields(stdioServer({ active: false }), status(), t);
    expect(new Map(disabled.map((field) => [field.label, field.value])).get(t("mcp.form.enabled"))).toBe("false");
  });
});
