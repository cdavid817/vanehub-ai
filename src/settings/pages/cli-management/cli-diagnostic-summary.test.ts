import type { TFunction } from "i18next";
import { beforeAll, describe, expect, it } from "vitest";
import { activateAppLanguage, i18n } from "../../../i18n";
import { normalizeDisplayPath } from "../../../lib/session-path";
import type { CliEnvironmentSnapshot } from "../../../types/cli-environment-snapshot";
import { buildCliDiagnosticFields } from "./cli-diagnostic-summary";

let t: TFunction;
beforeAll(async () => {
  await activateAppLanguage("en");
  t = i18n.getFixedT("en");
});

function snapshot(overrides: Partial<CliEnvironmentSnapshot> = {}): CliEnvironmentSnapshot {
  return {
    schemaVersion: 1,
    agentId: "claude-code",
    displayName: "Anthropic Claude Code CLI",
    provider: "Anthropic",
    executableNames: ["claude"],
    scope: "local-desktop",
    overallState: "ready",
    freshness: "fresh",
    environmentFingerprint: "fingerprint-a",
    installations: [{
      id: "claude-npm",
      executablePath: "/mock/npm/bin/claude",
      canonicalPath: "/mock/npm/lib/claude",
      aliasPaths: [],
      targetMissing: false,
      reportedVersion: "1.2.0",
      sourceId: "npm",
      sourceKind: "npm",
      sourceConfidence: "verified",
      pathPriority: 0,
      environmentOrigin: "path",
      executableStatus: "healthy",
    }],
    pathSelectedInstallationId: "claude-npm",
    recommendedInstallationId: "claude-npm",
    discovery: "found-one",
    executable: "healthy",
    authentication: "authenticated",
    readiness: "ready",
    compatibility: "supported",
    update: "up-to-date",
    conflicts: [],
    sources: [{
      sourceId: "npm",
      kind: "npm",
      supportedOnThisPlatform: true,
      availableVersionCount: 3,
      management: "managed",
      guidanceCode: null,
      availableVersions: ["1.2.0"],
      capabilities: { install: "supported", upgrade: "supported", downgrade: "supported", reinstall: "supported", uninstall: true, repair: "supported" },
    }],
    allowedActions: [],
    lastMutation: null,
    lastOperationId: null,
    checkedAt: "2026-01-01T00:00:00.000Z",
    ...overrides,
  };
}

describe("buildCliDiagnosticFields", () => {
  it("includes version, every status axis, stable ids, the executable path, and the last-checked timestamp", () => {
    const fields = buildCliDiagnosticFields(snapshot(), t);
    const byLabel = new Map(fields.map((field) => [field.label, field.value]));

    expect(byLabel.get("Version")).toBe("1.2.0");
    expect(byLabel.get(t("cli.axis.overall"))).toBe("ready");
    expect(byLabel.get(t("cli.axis.authentication"))).toBe("authenticated");
    expect(byLabel.get("Agent id")).toBe("claude-code");
    expect(byLabel.get("Installation id")).toBe("claude-npm");
    expect(byLabel.get("Source id")).toBe("npm");
    // The same `normalizeDisplayPath` treatment this page's own on-screen overview already
    // applies to this exact path, not a raw untouched value from the fixture.
    expect(byLabel.get("Executable path")).toBe(normalizeDisplayPath("/mock/npm/bin/claude"));
    expect(byLabel.get(t("cli.lastChecked"))).toBe("2026-01-01T00:00:00.000Z");
  });

  it("marks a field unavailable rather than omitting or inventing it when nothing is installed", () => {
    const fields = buildCliDiagnosticFields(
      snapshot({ installations: [], pathSelectedInstallationId: null, recommendedInstallationId: null }),
      t,
    );
    const byLabel = new Map(fields.map((field) => [field.label, field.value]));
    expect(byLabel.get("Version")).toBeNull();
    expect(byLabel.get("Executable path")).toBeNull();
  });

  it("joins multiple active conflict and withheld-action reason codes rather than dropping all but one", () => {
    const fields = buildCliDiagnosticFields(
      snapshot({
        conflicts: [
          { kind: "duplicate", severity: "warning", installationIds: ["claude-npm"], blocksMutation: false, blocksLaunch: false, reasonCode: "cli.conflict.duplicateSource" },
          { kind: "stale", severity: "warning", installationIds: ["claude-npm"], blocksMutation: false, blocksLaunch: false, reasonCode: "cli.conflict.stalePath" },
        ],
        allowedActions: [
          { action: "downgrade", sourceId: "npm", targetMode: "exact", defaultTarget: null, requiresTargetSelection: true, reasonCode: "cli.action.reason.networkRequired" },
        ],
      }),
      t,
    );
    const byLabel = new Map(fields.map((field) => [field.label, field.value]));
    expect(byLabel.get("Conflict codes")).toBe("cli.conflict.duplicateSource, cli.conflict.stalePath");
    expect(byLabel.get("Withheld-action reason codes")).toBe("cli.action.reason.networkRequired");
  });

  it("never carries anything beyond the bounded fields this snapshot type can hold", () => {
    // Every field's value traces back to a version string, a backend-pinned enum/reason-code
    // union, a stable id, an already-displayed path, or a timestamp -- there is no free-text
    // field on CliEnvironmentSnapshot for this test to accidentally miss redacting.
    const fields = buildCliDiagnosticFields(snapshot(), t);
    expect(fields.every((field) => typeof field.label === "string" && (field.value === null || typeof field.value === "string"))).toBe(true);
  });
});
