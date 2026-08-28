import { describe, expect, it } from "vitest";
import {
  webCliActionPlan,
  webCliBulkActionPlan,
  webCliBulkItemResults,
  webCliEnvironmentSnapshots,
  WEB_CLI_FIXED_PLAN_IDS,
  WEB_CLI_OUTCOME_TARGETS,
  WEB_CLI_REFUSAL_TARGETS,
} from "./web-cli-environment-fixtures";

/**
 * The Web/mock fixtures speak the same vocabulary the backend emits.
 *
 * TypeScript cannot check this: every enum-valued field on a snapshot is typed `string`, because
 * the values come from Rust `as_str` implementations rather than from a union declared here. A
 * value invented on this side compiles, serializes, reaches the UI, and renders as its own
 * localization key -- which is exactly how `found-many` sat in these fixtures while the backend
 * has only ever emitted `found-multiple`.
 */

const DISCOVERY = ["not-scanned", "not-found", "found-one", "found-multiple"];
const EXECUTABLE = [
  "not-applicable",
  "healthy",
  "broken",
  "timeout",
  "permission-denied",
  "unsupported-architecture",
  "unknown",
];
const AUTHENTICATION = ["authenticated", "required", "expired", "unknown", "not-applicable"];
const READINESS = ["ready", "needs-auth", "missing-dependency", "misconfigured", "broken", "unknown"];
const COMPATIBILITY = ["supported", "unsupported-version", "unsupported-platform", "unknown"];
const UPDATE = ["not-applicable", "up-to-date", "available", "ahead", "catalog-unavailable", "unknown"];
const FRESHNESS = ["never", "fresh", "stale", "refreshing"];
const OVERALL = ["broken", "conflict", "needs-auth", "update-available", "ready", "missing", "unknown"];
const CONFIDENCE = ["unknown", "inferred", "verified"];
const ORIGIN = ["path", "known-location"];
const SEVERITY = ["info", "warning", "error", "blocking"];
const SOURCE_KIND = [
  "npm",
  "winget",
  "vendor-installer",
  "homebrew",
  "bun",
  "volta",
  "desktop",
  "system",
  "manual",
  "unknown",
];
const CONFLICT_KIND = [
  "duplicate-launcher-alias",
  "path-shadowing",
  "broken-path-precedence",
  "multiple-installation-sources",
  "version-divergence",
  "ambiguous-source-ownership",
  "environment-path-divergence",
  "architecture-mismatch",
  "stale-launcher-target",
];
const ACTION = ["install", "upgrade", "downgrade", "reinstall", "uninstall", "repair"];
const TARGET_MODE = ["exact", "latest-only", "unsupported"];
const PLAN_STATE = ["draft", "executing", "completed", "failed", "cancelled", "expired"];

describe("Web CLI environment fixtures", () => {
  const snapshots = webCliEnvironmentSnapshots();

  it("covers the states a UI has to render", () => {
    expect(snapshots.map((snapshot) => snapshot.agentId)).toEqual([
      "claude-code",
      "codex-cli",
      "gemini-cli",
      "opencode",
      "antigravity-cli",
    ]);
    expect(snapshots.some((snapshot) => snapshot.conflicts.length > 0)).toBe(true);
    expect(snapshots.some((snapshot) => snapshot.installations.length === 0)).toBe(true);
    expect(snapshots.some((snapshot) => snapshot.freshness === "stale")).toBe(true);
    expect(snapshots.some((snapshot) =>
      snapshot.sources.some((source) => source.management === "detect-only"))).toBe(true);
    expect(snapshots.some((snapshot) =>
      snapshot.pathSelectedInstallationId !== snapshot.recommendedInstallationId)).toBe(true);
  });

  it("uses only values the backend emits for every enum-valued snapshot field", () => {
    for (const snapshot of snapshots) {
      const where = snapshot.agentId;
      expect(DISCOVERY, `${where}.discovery`).toContain(snapshot.discovery);
      expect(EXECUTABLE, `${where}.executable`).toContain(snapshot.executable);
      expect(AUTHENTICATION, `${where}.authentication`).toContain(snapshot.authentication);
      expect(READINESS, `${where}.readiness`).toContain(snapshot.readiness);
      expect(COMPATIBILITY, `${where}.compatibility`).toContain(snapshot.compatibility);
      expect(UPDATE, `${where}.update`).toContain(snapshot.update);
      expect(FRESHNESS, `${where}.freshness`).toContain(snapshot.freshness);
      expect(OVERALL, `${where}.overallState`).toContain(snapshot.overallState);

      for (const installation of snapshot.installations) {
        expect(SOURCE_KIND, `${where}.sourceKind`).toContain(installation.sourceKind);
        expect(CONFIDENCE, `${where}.sourceConfidence`).toContain(installation.sourceConfidence);
        expect(ORIGIN, `${where}.environmentOrigin`).toContain(installation.environmentOrigin);
        expect(EXECUTABLE, `${where}.executableStatus`).toContain(installation.executableStatus);
      }
      for (const conflict of snapshot.conflicts) {
        expect(CONFLICT_KIND, `${where}.conflict.kind`).toContain(conflict.kind);
        expect(CONFLICT_KIND, `${where}.conflict.reasonCode`).toContain(conflict.reasonCode);
        expect(SEVERITY, `${where}.conflict.severity`).toContain(conflict.severity);
      }
      for (const source of snapshot.sources) {
        expect(SOURCE_KIND, `${where}.source.kind`).toContain(source.kind);
      }
      for (const action of snapshot.allowedActions) {
        expect(ACTION, `${where}.action`).toContain(action.action);
        expect(TARGET_MODE, `${where}.targetMode`).toContain(action.targetMode);
      }
    }
  });

  it("keeps every installation and source cross-referenced inside its own snapshot", () => {
    for (const snapshot of snapshots) {
      const installationIds = new Set(snapshot.installations.map((item) => item.id));
      const sourceIds = new Set(snapshot.sources.map((item) => item.sourceId));
      if (snapshot.pathSelectedInstallationId) {
        expect(installationIds).toContain(snapshot.pathSelectedInstallationId);
      }
      if (snapshot.recommendedInstallationId) {
        expect(installationIds).toContain(snapshot.recommendedInstallationId);
      }
      for (const installation of snapshot.installations) {
        if (installation.sourceId) expect(sourceIds).toContain(installation.sourceId);
      }
      // An action offered for a source this tool does not have would be unactionable on screen.
      for (const action of snapshot.allowedActions) expect(sourceIds).toContain(action.sourceId);
      for (const conflict of snapshot.conflicts) {
        for (const id of conflict.installationIds) expect(installationIds).toContain(id);
      }
    }
  });

  it("never puts a path that could be read as a real host path on screen", () => {
    for (const snapshot of snapshots) {
      for (const installation of snapshot.installations) {
        expect(installation.executablePath.startsWith("/mock/")).toBe(true);
      }
    }
  });

  it("offers every sentinel target in the catalog the UI reads", () => {
    const claude = snapshots.find((snapshot) => snapshot.agentId === "claude-code");
    const versions = claude?.sources.find((source) => source.sourceId === "npm")?.availableVersions ?? [];
    // A sentinel absent from the catalog cannot be selected, so the outcome it drives is
    // unreachable from the UI even though the adapter still handles it.
    for (const target of [
      ...Object.values(WEB_CLI_OUTCOME_TARGETS),
      ...Object.values(WEB_CLI_REFUSAL_TARGETS),
    ]) {
      expect(versions, target).toContain(target);
    }
    expect(claude?.sources.find((source) => source.sourceId === "npm")?.availableVersionCount)
      .toBe(versions.length);
  });

  it("builds plans and bulk results in the shapes the contract declares", () => {
    const plan = webCliActionPlan({ id: WEB_CLI_FIXED_PLAN_IDS.draft });
    expect(PLAN_STATE).toContain(plan.state);
    expect(ACTION).toContain(plan.action);
    expect(plan.commandPreview.args.length).toBeGreaterThan(0);

    const bulk = webCliBulkActionPlan("web-bulk-plan");
    expect(bulk.id).toBe("web-bulk-plan");
    expect(bulk.items.length + bulk.skipped.length).toBeGreaterThan(1);

    const items = webCliBulkItemResults();
    // Both arms of the discriminated union, so a UI can be built against each.
    expect(items.some((item) => item.status === "completed")).toBe(true);
    expect(items.some((item) => item.status === "skipped")).toBe(true);
    for (const item of items) {
      if (item.status === "completed") expect(item.reason).toBeNull();
      else expect(item.outcome).toBeNull();
    }
  });
});
