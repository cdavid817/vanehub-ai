import type {
  EvolutionApplicationSummary,
  EvolutionBreakerSummary,
  EvolutionEligibilitySummary,
  EvolutionNotificationEvent,
  EvolutionPolicy,
  EvolutionProbationSummary,
  EvolutionRunSummary,
  SkillEvolutionOrchestrationService,
} from "./skill-evolution-orchestration-service";

const policies = new Map<string, EvolutionPolicy>();
const runs: EvolutionRunSummary[] = [];
const eligibility: EvolutionEligibilitySummary[] = [];
const applications: EvolutionApplicationSummary[] = [];
const probations: EvolutionProbationSummary[] = [];
const breakers: EvolutionBreakerSummary[] = [];
const notificationHandlers = new Set<(event: EvolutionNotificationEvent) => void>();
const notificationReceipts = new Set<string>();

function policy(workspaceId: string): EvolutionPolicy {
  return policies.get(workspaceId) ?? {
    workspaceId,
    mode: "off",
    allowedSkillIds: [],
    consent: null,
    revision: 0,
    updatedAtMs: 0,
    mockProvenance: "web_simulation",
  };
}

function page<T>(items: T[], cursor?: string, limit = 50) {
  const offset = cursor ? Number.parseInt(cursor, 10) : 0;
  if (!Number.isInteger(offset) || offset < 0 || limit < 1 || limit > 100) {
    throw new Error("invalid_input");
  }
  const bounded = offset;
  const selected = items.slice(bounded, bounded + limit);
  const next = bounded + selected.length;
  return { items: selected, nextCursor: next < items.length ? String(next) : null };
}

export const webSkillEvolutionOrchestrationClient: SkillEvolutionOrchestrationService = {
  async getEvolutionSchedulerOverview(workspaceId) {
    const currentPolicy = policy(workspaceId);
    const active = runs.find((run) => run.workspaceId === workspaceId && run.status === "running");
    return {
      workspaceId,
      mode: currentPolicy.mode,
      pendingTriggers: 0,
      activeRunId: active?.runId ?? null,
      idleGate: "ready",
      automaticMutationAvailable: false,
      triggerCounters: emptyTriggerCounters(),
      idle: { state: "ready", safeReasons: [] },
      mockProvenance: "web_simulation",
    };
  },
  async getEvolutionPolicy(workspaceId) {
    return policy(workspaceId);
  },
  async updateEvolutionPolicy(input) {
    const current = policy(input.workspaceId);
    if (current.revision !== input.expectedRevision) throw new Error("stale_conflict");
    if (input.allowedSkillIds.some((id) => id === "*" || id.trim().length === 0)) {
      throw new Error("invalid_allowlist");
    }
    const hasCurrentConsent = input.acknowledgeCurrentDisclosure || current.consent !== null;
    if (input.mode === "enabled" && (input.allowedSkillIds.length === 0 || !hasCurrentConsent)) {
      throw new Error("consent_and_allowlist_required");
    }
    const next: EvolutionPolicy = {
      workspaceId: input.workspaceId,
      mode: input.mode,
      allowedSkillIds: [...new Set(input.allowedSkillIds)].sort(),
      consent: input.acknowledgeCurrentDisclosure
        ? {
            disclosureVersion: "skill-evolution-orchestration-disclosure-v1",
            disclosureHash: `web-consent-${current.revision + 1}`,
            acceptedAtMs: Date.now(),
          }
        : current.consent,
      revision: current.revision + 1,
      updatedAtMs: Date.now(),
      mockProvenance: "web_simulation",
    };
    policies.set(input.workspaceId, next);
    return next;
  },
  async listEvolutionRuns(input) {
    return page(
      runs.filter((run) => run.workspaceId === input.workspaceId),
      input.cursor,
      input.limit,
    );
  },
  async getEvolutionRun(runId) {
    const run = runs.find((candidate) => candidate.runId === runId);
    if (!run) throw new Error("not_found");
    return { ...run, stages: [], checkpoints: [] };
  },
  async listEvolutionEligibility(input) {
    return page(eligibility.filter((item) => item.runId.startsWith(`${input.workspaceId}:`)), input.cursor, input.limit);
  },
  async listEvolutionApplications(input) {
    return page(applications.filter((item) => item.runId.startsWith(`${input.workspaceId}:`)), input.cursor, input.limit);
  },
  async listEvolutionProbations(input) {
    return page(probations.filter((item) => item.workspaceId === input.workspaceId), input.cursor, input.limit);
  },
  async listEvolutionBreakers(input) {
    return page(
      breakers.filter((breaker) => breaker.workspaceId === input.workspaceId),
      input.cursor,
      input.limit,
    );
  },
  async requestEvolutionRun(workspaceId) {
    const now = Date.now();
    const run: EvolutionRunSummary = {
      runId: `${workspaceId}:web-evolution-run-${now}-${runs.length + 1}`,
      workspaceId,
      status: "completed",
      currentStage: null,
      policyWitnessHash: `web-simulation-policy-${policy(workspaceId).revision}`,
      safeFailureCode: null,
      budget: webBudget(),
      usage: {
        elapsedMs: 1, evidenceItems: 0, seedGroups: 0, assessments: 0,
        modelCalls: 0, notifications: 0, automaticMutations: 0,
      },
      revision: 1,
      createdAtMs: now,
      updatedAtMs: now,
      mockProvenance: "web_simulation",
    };
    runs.unshift(run);
    simulateDecision(run, policy(workspaceId), now);
    return { requestId: run.runId, queued: true, mockProvenance: "web_simulation" };
  },
  async cancelEvolutionRun(runId, expectedRevision) {
    const run = runs.find((candidate) => candidate.runId === runId);
    if (!run) throw new Error("not_found");
    if (run.revision !== expectedRevision || run.status === "completed") {
      throw new Error("stale_conflict");
    }
    run.status = "cancelled";
    run.revision += 1;
    run.updatedAtMs = Date.now();
    return { runId: run.runId, status: run.status, revision: run.revision };
  },
  async acknowledgeEvolutionBreaker(breakerId, expectedRevision) {
    const breaker = breakers.find((candidate) => candidate.breakerId === breakerId);
    if (!breaker) throw new Error("not_found");
    if (
      breaker.revision !== expectedRevision ||
      breaker.status !== "awaiting_acknowledgement" ||
      !breaker.healthProbePassed
    ) {
      throw new Error("health_and_acknowledgement_required");
    }
    breaker.status = "closed";
    breaker.revision += 1;
    breaker.updatedAtMs = Date.now();
    return { ...breaker };
  },
  async subscribeEvolutionNotifications(handler) {
    notificationHandlers.add(handler);
    return () => notificationHandlers.delete(handler);
  },
};

export function resetWebSkillEvolutionOrchestrationForTest() {
  policies.clear();
  runs.splice(0);
  eligibility.splice(0);
  applications.splice(0);
  probations.splice(0);
  breakers.splice(0);
  notificationHandlers.clear();
  notificationReceipts.clear();
}

function simulateDecision(run: EvolutionRunSummary, current: EvolutionPolicy, now: number) {
  if (current.mode === "off" || current.allowedSkillIds.length === 0) return;
  const skillId = current.allowedSkillIds[0];
  const decision: EvolutionEligibilitySummary = {
    eligibilityId: `${run.runId}:eligibility`,
    runId: run.runId,
    draftId: `${run.runId}:draft`,
    targetSkillId: skillId,
    draftProvenance: "deterministic_authorized_correction",
    preflightState: current.mode === "enabled" ? "consumed" : "not_issued",
    result: current.mode === "observe" ? "would_apply" : "eligible",
    proofHash: `${run.runId}:proof`,
    overlayPreviewHash: `${run.runId}:simulated-preview`,
    evaluatedAtMs: now,
    predicates: [{ condition: "web_simulation", passed: true, safeReasonCode: null, witnessHash: null }],
    mockProvenance: "web_simulation",
  };
  eligibility.unshift(decision);
  if (current.mode !== "enabled") return;
  const applicationId = `${run.runId}:application`;
  applications.unshift({
    applicationId,
    runId: run.runId,
    eligibilityId: decision.eligibilityId,
    targetSkillId: skillId,
    curatorApplicationId: `${applicationId}:curator`,
    overlayApplicationId: `${applicationId}:overlay-simulation`,
    actor: "system_policy",
    committedAtMs: now,
    mockProvenance: "web_simulation",
  });
  probations.unshift({
    probationId: `${applicationId}:probation`,
    applicationId,
    workspaceId: current.workspaceId,
    skillId,
    status: "active",
    startsAtMs: now,
    endsAtMs: now + 7 * 24 * 60 * 60 * 1_000,
    revision: 1,
    mockProvenance: "web_simulation",
  });
  publishNotification({
    schemaVersion: 1, eventId: `automatic_application:${applicationId}`,
    eventKind: "automatic_application", workspaceId: current.workspaceId,
    runId: run.runId, applicationId, probationId: `${applicationId}:probation`,
    breakerId: null, skillId, safeReasonCode: null,
    probationEndsAtMs: now + 7 * 24 * 60 * 60 * 1_000, entityRevision: 0,
    mockProvenance: "web_simulation",
  });
}

export function seedWebEvolutionBreakerForTest(workspaceId: string) {
  const now = Date.now();
  const breaker: EvolutionBreakerSummary = {
    breakerId: `${workspaceId}:web-breaker`, workspaceId, skillId: null,
    status: "awaiting_acknowledgement", safeCauseCode: "simulated_failure",
    healthCheckVersion: "web-simulation-v1", healthProbePassed: true,
    revision: 1, updatedAtMs: now, mockProvenance: "web_simulation",
  };
  breakers.unshift(breaker);
  publishNotification({
    schemaVersion: 1, eventId: `breaker_recovered:${breaker.breakerId}:1`,
    eventKind: "breaker_recovered", workspaceId, runId: null, applicationId: null,
    probationId: null, breakerId: breaker.breakerId, skillId: null,
    safeReasonCode: breaker.safeCauseCode, probationEndsAtMs: null, entityRevision: 1,
    mockProvenance: "web_simulation",
  });
  return breaker;
}

function publishNotification(event: EvolutionNotificationEvent) {
  if (notificationReceipts.has(event.eventId)) return;
  notificationReceipts.add(event.eventId);
  for (const handler of notificationHandlers) {
    try {
      handler(structuredClone(event));
    } catch {
      // Notification consumers cannot affect simulated orchestration state.
    }
  }
}

function emptyTriggerCounters() {
  return {
    startupRecovery: 0, periodicMaintenance: 0, applicationIdleTransition: 0,
    agentRunCompletion: 0, conversationCompletion: 0, explicitFeedbackCommit: 0,
    verificationCompletion: 0, delegatedUtilityCompletion: 0,
    relevantPolicyOrSkillChange: 0, manualRunRequest: 0,
  };
}

function webBudget() {
  return {
    wallTimeMs: 300_000, evidenceItems: 5_000, seedGroups: 500,
    assessments: 100, modelCalls: 25, notifications: 50, automaticMutations: 1,
  };
}
