import type {
  AssessmentCheck,
  AssessmentDetail,
  AssessmentRoute,
  AssessmentSummary,
  SkillAssessmentService,
} from "./skill-assessment-service";

let consentEnabled = false;
const scheduledSeeds = new Map<string, string>();

export function resetWebSkillAssessmentForTest() {
  consentEnabled = false;
  scheduledSeeds.clear();
}

const routes: Record<string, AssessmentRoute> = {
  deterministic: "advance",
  model_assisted: "needs_human_review",
  fallback: "needs_human_review",
  ambiguous: "needs_human_review",
  superseded: "drop",
  privacy: "drop",
  duplicate: "merge_duplicate",
  transient: "record_memory_only",
  contradiction: "needs_human_review",
  executable: "needs_human_review",
  pinned: "record_memory_only",
  archived: "drop",
  model_invalid: "needs_human_review",
};

function scenarioFrom(value?: string) {
  const stored = typeof localStorage === "undefined" ? null : localStorage.getItem("vanehub.skillAssessmentScenario");
  const scenario = value?.startsWith("mock://") ? value.slice(7) : stored;
  return scenario?.replaceAll("-", "_") ?? "deterministic";
}

function summary(scenario: string): AssessmentSummary {
  const status = scenario === "pending" ? "pending" : scenario === "failed" ? "failed" : scenario === "superseded" ? "superseded" : "completed";
  return {
    attemptId: `mock-assessment-${scenario}`,
    seedId: "mock-seed-1",
    seedRevision: scenario === "changed_evidence" ? "revision-2" : "revision-1",
    status,
    ...(status === "completed" || status === "superseded" ? {
      classification: ["ambiguous", "fallback", "model_invalid"].includes(scenario) ? "ambiguous" as const : "selected" as const,
      route: routes[scenario] ?? "advance",
      confidence: ["ambiguous", "fallback", "model_invalid"].includes(scenario) ? "medium" as const : "high" as const,
      risk: ["privacy", "executable", "archived"].includes(scenario) ? "high" as const
        : ["model_assisted", "contradiction", "duplicate", "transient", "pinned"].includes(scenario) ? "medium" as const : "low" as const,
      winningRule: scenario === "fallback" ? "deterministic_fallback" : "quality_gate_lattice",
    } : {}),
    isCurrent: scenario !== "superseded",
    createdAtMs: 1_776_000_000_000,
    ...(status === "completed" || status === "superseded" ? { completedAtMs: 1_776_000_001_000 } : {}),
    ...(scenario === "superseded" ? { supersededByAttemptId: "mock-assessment-deterministic" } : {}),
    ...(scenario === "superseded" ? {
      supersessionReason: "selector_policy_changed",
      changedWitnessHash: "witness-hash-2",
    } : {}),
  };
}

function checkFixture(kind: string, scenario: string): Pick<AssessmentCheck, "result" | "severity" | "reasonCode" | "routeConstraints"> {
  const affected: Record<string, { kind: string; route: AssessmentRoute; severity: "medium" | "high"; reason: string; result: "fail" | "review" }> = {
    privacy: { kind: "privacy_residue", route: "drop", severity: "high", reason: "privacy_residue_detected", result: "fail" },
    duplicate: { kind: "duplicate_knowledge", route: "merge_duplicate", severity: "medium", reason: "canonical_duplicate", result: "review" },
    transient: { kind: "transient_incident", route: "record_memory_only", severity: "medium", reason: "workspace_local_fact", result: "review" },
    contradiction: { kind: "evidence_consistency", route: "needs_human_review", severity: "medium", reason: "material_contradiction", result: "review" },
    executable: { kind: "executable_content_risk", route: "needs_human_review", severity: "high", reason: "executable_expansion", result: "review" },
    pinned: { kind: "target_lifecycle_mutability", route: "record_memory_only", severity: "medium", reason: "target_pinned", result: "review" },
    archived: { kind: "target_lifecycle_mutability", route: "drop", severity: "high", reason: "target_archived", result: "fail" },
  };
  const fixture = affected[scenario];
  if (!fixture || fixture.kind !== kind) return { result: "pass", severity: "low", reasonCode: "check_passed", routeConstraints: [] };
  return { result: fixture.result, severity: fixture.severity, reasonCode: fixture.reason, routeConstraints: [fixture.route] };
}

function detail(scenario: string): AssessmentDetail {
  const base = summary(scenario);
  const completed = base.status === "completed" || base.status === "superseded";
  return {
    ...base,
    targets: completed ? [{
      ordinal: 0,
      skillId: "review",
      skillType: scenario === "attribution_utility" ? "utility" : "role",
      revisionHash: scenario === "changed_revision" ? "revision-hash-2" : "revision-hash-1",
      scope: "project",
      lifecycle: scenario === "pinned" ? "pinned" : scenario === "archived" ? "archived" : "active",
      trust: "trusted",
      score: ["ambiguous", "fallback", "model_invalid"].includes(scenario) ? 64 : 91,
      attribution: scenario === "attribution_managed" ? "correlated" : scenario === "attribution_interactive" ? "weak" : "verified",
      attributionUncertain: ["ambiguous", "attribution_managed", "attribution_interactive"].includes(scenario),
      matchedFeatureClasses: ["capability", "description"],
      exclusions: [],
      components: [
        { component: "attribution", score: 35 },
        { component: "participation", score: 15 },
        { component: "compatibility", score: 20 },
        { component: "lexical", score: 16 },
        { component: "locality", score: 5 },
      ],
    }] : [],
    checks: completed ? [
      "privacy_residue",
      "evidence_sufficiency",
      "duplicate_knowledge",
      "transient_incident",
      "guidance_specificity",
      "evidence_consistency",
      "target_compatibility",
      "executable_content_risk",
      "target_lifecycle_mutability",
    ].map((kind, ordinal) => ({
      ordinal,
      kind,
      result: checkFixture(kind, scenario).result,
      severity: checkFixture(kind, scenario).severity,
      reasonCode: checkFixture(kind, scenario).reasonCode,
      evidenceIds: ["mock-signal-1"],
      routeConstraints: checkFixture(kind, scenario).routeConstraints,
    })) : [],
    provenance: {
      deterministic: true,
      modelEvaluationAllowed: consentEnabled || scenario === "model_assisted",
      modelConsulted: scenario === "model_assisted",
      ...(["fallback", "model_invalid"].includes(scenario) ? { fallbackReason: scenario === "model_invalid" ? "invalid_schema" : "provider_unavailable" } : {}),
      ...(scenario === "model_assisted" ? {
        providerProtocol: "openai-compatible",
        modelId: "mock-judge",
        templateVersion: "assessment-v1",
        responseSchemaVersion: "quality-v1",
      } : {}),
    },
    routeConstraints: scenario === "ambiguous" ? ["needs_human_review"] : [],
    ...(completed ? {
      selectionThreshold: {
        leadingScore: ["ambiguous", "fallback", "model_invalid"].includes(scenario) ? 64 : 91,
        runnerUpScore: ["ambiguous", "fallback", "model_invalid"].includes(scenario) ? 58 : 52,
        margin: ["ambiguous", "fallback", "model_invalid"].includes(scenario) ? 6 : 39,
        selectedMinimum: 60,
        ambiguousMinimum: 45,
        requiredMargin: 15,
      },
    } : {}),
    versionWitnesses: {
      witnessHash: "witness-hash-1",
      lineageHash: "lineage-hash-1",
      targetUniverseHash: "target-universe-hash-1",
      sanitizerVersion: "sanitizer-v1",
      selectorPolicyVersion: scenario === "policy_upgrade" ? "selector-v2" : "selector-v1",
      gatePolicyVersion: "gates-v1",
      routingPolicyVersion: "routing-v1",
      confidencePolicyVersion: "confidence-v1",
      consentVersion: consentEnabled ? "assessment-disclosure-v1" : "disabled",
    },
  };
}

export const webSkillAssessmentClient: SkillAssessmentService = {
  async querySkillEvolutionAssessments(input) {
    const scenario = scenarioFrom(input.workspace);
    const all = ["history", "changed_evidence", "changed_revision", "policy_upgrade", "consent_revocation"].includes(scenario)
      ? [summary(scenario === "history" ? "deterministic" : scenario), summary("superseded")]
      : [summary(scenario)];
    const offset = Number.parseInt(input.cursor?.replace("mock-", "") ?? "0", 10) || 0;
    const limit = Math.min(Math.max(input.limit ?? 20, 1), 100);
    const items = all.slice(offset, offset + limit);
    return {
      items,
      ...(offset + items.length < all.length ? { nextCursor: `mock-${offset + items.length}` } : {}),
    };
  },

  async getSkillEvolutionAssessment(attemptId) {
    if (!attemptId.startsWith("mock-assessment-")) return null;
    return detail(attemptId.slice("mock-assessment-".length));
  },

  async getSkillEvolutionAssessmentPolicy() {
    const scenario = scenarioFrom();
    return {
      evaluatorPolicyVersion: "structured-evaluator-v1",
      disclosureVersion: "assessment-disclosure-v1",
      modelEvaluationEnabled: scenario === "consent_revocation" ? false : consentEnabled,
      providerAvailable: scenario !== "provider_unavailable",
      changedAtMs: consentEnabled ? 1_776_000_002_000 : 0,
    };
  },

  async updateSkillEvolutionAssessmentConsent(input) {
    if (input.evaluatorPolicyVersion !== "structured-evaluator-v1" || input.disclosureVersion !== "assessment-disclosure-v1") {
      throw new Error("Assessment consent version is stale");
    }
    consentEnabled = input.enabled;
    return this.getSkillEvolutionAssessmentPolicy();
  },

  async scheduleSkillEvolutionReassessment(input) {
    const existing = scheduledSeeds.get(input.seedId);
    if (existing) return { queueId: existing, status: "coalesced" };
    const queueId = `mock-reassessment-${input.seedId}`;
    scheduledSeeds.set(input.seedId, queueId);
    return { queueId, status: "scheduled" };
  },
};
