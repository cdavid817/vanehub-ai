import type { AgentService } from "./agent-service";
import type { SkillGovernanceService } from "./skill-governance-service";
import { normalizeWebSkillLocation } from "./web-skill-location";
import type { SkillDriftReport, SkillSyncResult } from "../types/skill";
import type { SkillToolRevision } from "../types/skill-tools";

const webSkillToolInspection: SkillToolRevision[] = [
  {
    skillId: "code-review",
    toolId: "inspect-diff",
    canonicalId: `skill__code-review__inspect-diff__${"f".repeat(12)}`,
    revision: "f".repeat(64),
    sourceScope: "global",
    implementationKind: "declarative",
    baseRevision: "web-inspection-only",
    manifestHash: `sha256:${"a".repeat(64)}`,
    implementationHash: `sha256:${"b".repeat(64)}`,
    capabilityDigest: "web-inspection-only",
    validation: "valid",
    trusted: false,
    enabled: false,
    quarantined: false,
    consecutiveFailures: 0,
    diagnostics: [],
    runtimeSupport: "unsupported-web-runtime",
    enforcementStrength: "bounded-native-io",
    createdAt: "2026-01-01T00:00:00Z",
    updatedAt: "2026-01-01T00:00:00Z",
  },
];

function webSkillToolUnsupported(): never {
  throw new Error("unsupported-web-runtime");
}

function webSkillToolByRevision(revision: string): SkillToolRevision {
  const tool = webSkillToolInspection.find((candidate) => candidate.revision === revision);
  if (!tool) throw new Error("skill-tool-not-found");
  return structuredClone(tool);
}

export const webSkillGovernanceClient: SkillGovernanceService = {
  async listSkillTools(input) {
    return webSkillToolInspection
      .filter(
        (tool) =>
          tool.skillId === input.skillId &&
          tool.sourceScope === input.scope &&
          (tool.workspacePath ?? null) === (input.workspacePath ?? null),
      )
      .map((tool) => structuredClone(tool));
  },

  async validateSkillToolRevision() {
    return webSkillToolUnsupported();
  },

  async setSkillToolTrust() {
    return webSkillToolUnsupported();
  },

  async setSkillToolEnabled() {
    return webSkillToolUnsupported();
  },

  async quarantineSkillTool() {
    return webSkillToolUnsupported();
  },

  async recoverSkillTool() {
    return webSkillToolUnsupported();
  },

  async getSkillToolDiagnostics(input) {
    return webSkillToolByRevision(input.revision);
  },

  async detectSkillDrift(input): Promise<SkillDriftReport> {
    const location = normalizeWebSkillLocation(input);
    const issues: SkillDriftReport["issues"] = [];
    return {
      scope: location.scope,
      workspacePath: location.workspacePath ?? null,
      issues,
      driftHash: `web-${issues.length}`,
    };
  },

  async syncSkillDrift(this: AgentService, input): Promise<SkillSyncResult> {
    const report = await this.detectSkillDrift(input);
    return {
      mounted: [],
      unmounted: [],
      overwritten: [],
      backedUp: [],
      restored: [],
      failed: [],
      resolvedFrom: report,
    };
  },
};
