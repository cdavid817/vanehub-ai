import type { TFunction } from "i18next";
import { beforeAll, describe, expect, it } from "vitest";
import { activateAppLanguage, i18n } from "../../../i18n";
import { formatDiagnosticSummary } from "../../../ui/diagnostics/diagnostic-field";
import type { SshConnection } from "../../../types/ssh-connection";
import { buildSshConnectionDiagnosticFields } from "./ssh-connection-diagnostic-summary";

let t: TFunction;
beforeAll(async () => {
  await activateAppLanguage("en");
  t = i18n.getFixedT("en");
});

function connection(overrides: Partial<SshConnection> = {}): SshConnection {
  return {
    id: "conn-1",
    name: "Deploy box",
    host: "10.0.0.5",
    port: 22,
    user: "deploy",
    defaultPath: "/srv/app",
    authMode: "key",
    keyPath: "~/.ssh/id_ed25519",
    hasPassword: false,
    revision: 3,
    hostTrust: null,
    testStatus: "succeeded",
    lastConnectedAt: "2026-08-01T00:00:00Z",
    lastError: null,
    createdAt: "2026-01-01T00:00:00Z",
    updatedAt: "2026-08-01T00:00:00Z",
    ...overrides,
  };
}

// `SshConnection` has no password/key-content field at all to plant a leak in (unlike IM's
// `publicConfig`, which can carry a secret string alongside a safe one in the same dict) -- the
// realistic leak risk here is the connection's own private external identifiers, not a credential
// value, so these fixtures stand in for "a value that looks worth protecting" the same way IM's
// SECRET_VALUE constant does.
const SECRET_HOST = "10.55.22.9";
const SECRET_USER = "root-admin";
const SECRET_KEY_PATH = "~/.ssh/id_rsa_prod_db_backup";
const SECRET_PATH = "/srv/customer-records";
const SECRET_NAME = "Prod DB (do not share)";

describe("buildSshConnectionDiagnosticFields (redaction)", () => {
  it("never includes host, user, key path, default path, or the connection's own free-text name", () => {
    const fields = buildSshConnectionDiagnosticFields(
      connection({ host: SECRET_HOST, user: SECRET_USER, keyPath: SECRET_KEY_PATH, defaultPath: SECRET_PATH, name: SECRET_NAME }),
      t,
    );
    const summary = formatDiagnosticSummary(fields, "unavailable");
    expect(summary).not.toContain(SECRET_HOST);
    expect(summary).not.toContain(SECRET_USER);
    expect(summary).not.toContain(SECRET_KEY_PATH);
    expect(summary).not.toContain(SECRET_PATH);
    expect(summary).not.toContain(SECRET_NAME);

    // Matching im-diagnostic-summary.test.ts's own rigor: the excluded fields' labels must not
    // appear either -- "unavailable" is reserved for a field that applies but has no value right
    // now, not a stand-in for a value this builder refuses to reveal.
    expect(fields.some((field) => field.label === t("sshConnections.fields.host"))).toBe(false);
    expect(fields.some((field) => field.label === t("sshConnections.fields.user"))).toBe(false);
    expect(fields.some((field) => field.label === t("sshConnections.fields.keyPath"))).toBe(false);
    expect(fields.some((field) => field.label === t("sshConnections.fields.defaultPath"))).toBe(false);
    expect(fields.some((field) => field.label === t("sshConnections.fields.name"))).toBe(false);
  });

  it("reports hasPassword as a boolean flag, never a password value", () => {
    const withPassword = buildSshConnectionDiagnosticFields(connection({ authMode: "password", hasPassword: true }), t);
    const byLabelTrue = new Map(withPassword.map((field) => [field.label, field.value]));
    expect(byLabelTrue.get(t("sshConnections.diagnostics.field.hasPassword"))).toBe("true");

    const withoutPassword = buildSshConnectionDiagnosticFields(connection({ authMode: "key", hasPassword: false }), t);
    const byLabelFalse = new Map(withoutPassword.map((field) => [field.label, field.value]));
    expect(byLabelFalse.get(t("sshConnections.diagnostics.field.hasPassword"))).toBe("false");
  });

  it("includes the connection id, auth mode, test status, and timestamps as raw backend values", () => {
    const fields = buildSshConnectionDiagnosticFields(
      connection({
        id: "conn-42",
        authMode: "key",
        testStatus: "failed",
        createdAt: "2026-01-01T00:00:00Z",
        updatedAt: "2026-02-01T00:00:00Z",
        lastConnectedAt: "2026-02-15T00:00:00Z",
      }),
      t,
    );
    const byLabel = new Map(fields.map((field) => [field.label, field.value]));
    expect(byLabel.get(t("sshConnections.diagnostics.field.id"))).toBe("conn-42");
    expect(byLabel.get(t("sshConnections.fields.authMode"))).toBe("key");
    expect(byLabel.get(t("sshConnections.diagnostics.field.testStatus"))).toBe("failed");
    expect(byLabel.get(t("sshConnections.diagnostics.field.createdAt"))).toBe("2026-01-01T00:00:00Z");
    expect(byLabel.get(t("sshConnections.diagnostics.field.updatedAt"))).toBe("2026-02-01T00:00:00Z");
    expect(byLabel.get(t("sshConnections.diagnostics.field.lastConnectedAt"))).toBe("2026-02-15T00:00:00Z");
  });

  it("marks lastConnectedAt unavailable rather than guessing when the connection has never connected", () => {
    const fields = buildSshConnectionDiagnosticFields(connection({ lastConnectedAt: null, testStatus: "not-tested" }), t);
    const byLabel = new Map(fields.map((field) => [field.label, field.value]));
    expect(byLabel.get(t("sshConnections.diagnostics.field.lastConnectedAt"))).toBeNull();
  });

  it("never carries anything beyond the bounded fields this connection type can hold", () => {
    const fields = buildSshConnectionDiagnosticFields(connection(), t);
    expect(fields.every((field) => typeof field.label === "string" && (field.value === null || typeof field.value === "string"))).toBe(true);
  });
});
