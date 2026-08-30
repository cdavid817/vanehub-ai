import type {
  DossierSectionPage,
  GenerationArtifactKind,
  GenerationJobDetail,
  GenerationPolicy,
  GenerationStageKind,
  GenerationUsage,
  SkillGenerationService,
} from "./skill-generation-service";
import { publishWebGenerationNotification, resetWebGenerationNotifications, subscribeWebGenerationNotifications } from "./web-skill-generation-notifications";
const stages: GenerationStageKind[] = [
  "freeze_input",
  "inspect_target",
  "build_dossier",
  "plan_mutation",
  "synthesize_structured_draft",
  "validate_and_simulate",
  "package_for_governance",
];
const dossierKinds = [
  "identity_and_provenance", "executive_summary", "candidate_seed",
  "source_signal_inventory", "attribution_and_target_selection",
  "assessment_and_quality_gates", "current_effective_skill_snapshot",
  "relevant_guidance_and_resource_context", "failure_recovery_and_verification_timeline",
  "privacy_and_redaction_report", "proposed_mutation_rationale", "verification_plan",
  "lineage_and_version_witnesses",
];
const policies = new Map<string, GenerationPolicy>();
const jobs = new Map<string, GenerationJobDetail>();
let sequence = 0;
const usage = (repair = false): GenerationUsage => ({
  elapsedMs: repair ? 1_400 : 900,
  modelCalls: repair ? 3 : 2,
  toolCalls: 5,
  inputTokens: 900,
  outputTokens: 320,
  validationRepairs: repair ? 1 : 0,
});

function policy(workspaceId: string): GenerationPolicy {
  return policies.get(workspaceId) ?? {
    workspaceId,
    enabled: false,
    disclosureVersion: "generation-disclosure-v1",
    allowedArtifactKinds: ["overlay_learn_block", "overlay_exact_patch", "new_skill"],
    dailyModelCalls: 12,
    dailyInputTokens: 80_000,
    dailyOutputTokens: 24_000,
    failedCancelledRetentionDays: 180,
    completedPackageRetentionDays: 365,
    revision: 1,
    policyHash: `mock-policy:${workspaceId}:1`,
  };
}
function artifactKind(index: number): GenerationArtifactKind {
  return (["overlay_learn_block", "overlay_exact_patch", "new_skill"] as const)[index % 3];
}

function buildJob(
  jobId: string,
  requestId: string,
  workspaceId: string,
  kind: GenerationArtifactKind,
  supersedesJobId?: string,
): GenerationJobDetail {
  const repair = requestId.includes("repair");
  const running = requestId.includes("running");
  const timestamp = new Date(1_800_000_000_000 + sequence * 1_000).toISOString();
  const completedStages = running ? 3 : stages.length;
  return {
    jobId,
    requestId,
    workspaceId,
    seedId: `seed-${jobId}`,
    assessmentAttemptId: `assessment-${jobId}`,
    status: running ? "running" : "completed",
    artifactKind: kind,
    currentStage: running ? stages[completedStages] : undefined,
    usage: usage(repair),
    handoffStatus: "pending",
    inputWitnessHash: `mock-input-witness:${jobId}`,
    supersedesJobId,
    createdAt: timestamp,
    updatedAt: timestamp,
    stages: stages.map((stage, index) => ({
      attemptId: `${jobId}:${stage}:${repair && index === 4 ? 2 : 1}`,
      stage,
      attempt: repair && index === 4 ? 2 : 1,
      status: index < completedStages ? "succeeded" : index === completedStages ? "running" : "pending",
      inputHash: `mock-input:${jobId}:${stage}`,
      outputHash: index < completedStages ? `mock-output:${jobId}:${stage}` : undefined,
      usage: index < completedStages ? usage(repair && index === 4) : usage(false),
      startedAt: timestamp,
      completedAt: index < completedStages ? timestamp : undefined,
    })),
    dossierId: `dossier-${jobId}`,
    dossierRevision: 1,
    dossierHash: `mock-dossier:${jobId}`,
    draftId: `draft-${jobId}`,
    artifactHash: `mock-artifact:${jobId}`,
    validationId: `validation-${jobId}`,
    previewWitnessHash: `mock-preview:${jobId}`,
    draft: {
      draftId: `draft-${jobId}`,
      generationAttempt: 1,
      artifactKind: kind,
      mediaType: "text/markdown",
      renderedContent: kind === "overlay_exact_patch"
        ? "- previous guidance\n+ evidence-bound guidance"
        : kind === "new_skill"
          ? "---\nname: generated-skill\ndescription: Mock sanitized proposal\n---\n\nFollow the verified workflow."
          : "Use the evidence-bound workflow and verify the result.",
      sizeBytes: 96,
      contentHash: `mock-artifact:${jobId}`,
      permanentlyManual: true,
      citations: [{ claimId: "claim-1", dossierSection: "assessment_and_quality_gates", sourceId: `assessment-${jobId}` }],
    },
    validation: {
      validationId: `validation-${jobId}`,
      status: "passed",
      checks: [{ code: "citation_integrity", status: "passed" }],
      previewWitnessHash: `mock-preview:${jobId}`,
      reportHash: `mock-validation:${jobId}`,
      repairAttempt: repair ? 1 : 0,
    },
    permanentlyManual: true,
  };
}

function clone<T>(value: T): T {
  return structuredClone(value);
}
export function resetWebSkillGenerationForTest(): void {
  policies.clear();
  jobs.clear();
  sequence = 0;
  resetWebGenerationNotifications();
}

export function seedWebGenerationJobForTest(requestId = "seeded-running"): GenerationJobDetail {
  sequence += 1;
  const job = buildJob(`mock-generation-${sequence}`, requestId, "workspace-one", artifactKind(sequence));
  jobs.set(job.jobId, job);
  return clone(job);
}

export const webSkillGenerationClient: SkillGenerationService = {
  async getGenerationPolicy(workspaceId) {
    return clone(policy(workspaceId));
  },

  async updateGenerationPolicy(input) {
    const current = policy(input.workspaceId);
    if (input.expectedRevision !== current.revision) throw new Error("generation-policy-conflict");
    if (input.disclosureVersion !== "generation-disclosure-v1") {
      throw new Error("generation-disclosure-required");
    }
    if (input.allowedArtifactKinds.length < 1 || input.allowedArtifactKinds.length > 3
      || new Set(input.allowedArtifactKinds).size !== input.allowedArtifactKinds.length) {
      throw new Error("generation-invalid-request");
    }
    if (input.enabled && (!input.providerProfileId || !input.modelId)) {
      throw new Error("generation-provider-not-ready");
    }
    const next: GenerationPolicy = {
      ...current,
      ...input,
      allowedArtifactKinds: [...input.allowedArtifactKinds],
      revision: current.revision + 1,
      policyHash: `mock-policy:${input.workspaceId}:${current.revision + 1}`,
    };
    policies.set(input.workspaceId, next);
    return clone(next);
  },

  async listGenerationJobs(input) {
    if (jobs.size === 0 && input.workspaceId === "mock://generation") {
      sequence += 1;
      const running = buildJob(`mock-generation-${sequence}`, "mock-running", input.workspaceId, "overlay_learn_block");
      sequence += 1;
      const reviewable = buildJob(`mock-generation-${sequence}`, "mock-reviewable", input.workspaceId, "new_skill");
      jobs.set(running.jobId, running);
      jobs.set(reviewable.jobId, reviewable);
    }
    const offset = Number(input.cursor ?? "0");
    const limit = input.limit ?? 20;
    if (!Number.isSafeInteger(offset) || offset < 0 || limit < 1 || limit > 100) {
      throw new Error("generation-invalid-request");
    }
    const filtered = [...jobs.values()].filter((job) =>
      (!input.workspaceId || job.workspaceId === input.workspaceId)
      && (!input.status || job.status === input.status));
    return {
      items: clone(filtered.slice(offset, offset + limit)),
      nextCursor: offset + limit < filtered.length ? String(offset + limit) : undefined,
    };
  },

  async getGenerationJob(jobId) {
    return jobs.has(jobId) ? clone(jobs.get(jobId)!) : null;
  },

  async cancelGenerationJob(jobId) {
    const current = jobs.get(jobId);
    if (!current) throw new Error("generation-job-not-found");
    if (current.status !== "running" && current.status !== "queued") {
      throw new Error("generation-job-immutable");
    }
    current.status = "cancelled";
    current.currentStage = undefined;
    current.safeFailureCode = "generation_cancelled_by_user";
    publishWebGenerationNotification(current, "cancelled");
    return clone(current);
  },

  async regenerateGenerationJob(input) {
    const prior = jobs.get(input.jobId);
    if (!prior) throw new Error("generation-job-not-found");
    if (input.expectedInputWitnessHash !== `mock-input-witness:${prior.jobId}`) {
      throw new Error("generation-stale-witness");
    }
    sequence += 1;
    const next = buildJob(
      `mock-generation-${sequence}`,
      input.requestId,
      prior.workspaceId ?? "global",
      prior.artifactKind ?? "overlay_learn_block",
      prior.jobId,
    );
    prior.status = "superseded";
    publishWebGenerationNotification(prior, "superseded");
    jobs.set(next.jobId, next);
    return clone(next);
  },

  async getGenerationDossierSection(dossierId, ordinal, cursor, limit = 20) {
    if (ordinal < 0 || ordinal >= dossierKinds.length || limit < 1 || limit > 100) {
      throw new Error("generation-invalid-dossier-query");
    }
    const offset = Number(cursor ?? "0");
    const records = Array.from({ length: 3 }, (_, index) => ({
      recordId: `${dossierId}:${ordinal}:${index}`,
      safeSummary: `Mock sanitized record ${index + 1}`,
    }));
    const page = records.slice(offset, offset + limit);
    return clone({
      dossierId, dossierRevision: 1, ordinal, kind: dossierKinds[ordinal], status: "complete",
      records: page, sourceWitnesses: [{ witnessHash: `mock-source:${dossierId}:${ordinal}` }],
      sourceLinks: [{ linkKind: "job", linkedId: dossierId.replace("dossier-", ""), linkedRevision: "1", witnessHash: `mock-link:${dossierId}` }],
      truncation: { complete: true, omittedRecords: 0, omittedBytes: 0 },
      sectionHash: `mock-section:${dossierId}:${ordinal}`,
      nextCursor: offset + limit < records.length ? String(offset + limit) : undefined,
      pageComplete: offset + limit >= records.length,
    } satisfies DossierSectionPage);
  },

  async getGenerationProvenance(jobId) {
    if (!jobs.has(jobId)) throw new Error("generation-job-not-found");
    return {
      jobId,
      modelCalls: [{ purpose: "skill_evolution_generation", outcome: "valid", inputTokens: 900 }],
      toolReceipts: [{ toolName: "read_dossier_section", outcome: "succeeded" }],
      validations: [{ status: "passed", permanentlyManual: true }],
    };
  },

  async listGenerationQuarantine(input) {
    const items = [...jobs.values()].filter((job) => job.artifactKind === "new_skill").map((job) => ({
      proposalId: `proposal-${job.jobId}`, jobId: job.jobId, status: "quarantined",
      candidateId: `generated-${job.jobId}`, scope: "project" as const,
      workspaceId: job.workspaceId, artifactHash: job.artifactHash ?? "", catalogWitnessHash: "mock-catalog", revision: 1,
    }));
    const offset = Number(input.cursor ?? "0");
    const limit = input.limit ?? 20;
    if (!Number.isSafeInteger(offset) || offset < 0 || limit < 1 || limit > 100) {
      throw new Error("generation-invalid-request");
    }
    return {
      items: clone(items.slice(offset, offset + limit)),
      nextCursor: offset + limit < items.length ? String(offset + limit) : undefined,
    };
  },

  async exportGenerationDossier(input) {
    if (!input.dossierId) throw new Error("generation-invalid-export");
    return { exportId: `mock-export-${input.dossierId}`, status: "exported", contentHash: `mock-export:${input.format}`, sizeBytes: 512, exportedFileRemainsUserManaged: true };
  },

  async handoffGenerationPackage(jobId) {
    const job = jobs.get(jobId);
    if (!job || job.status !== "completed") throw new Error("generation-handoff-not-ready");
    job.handoffStatus = "delivered";
    publishWebGenerationNotification(job, "review_ready");
    return clone(job);
  },

  async subscribeGenerationNotifications(handler) {
    return subscribeWebGenerationNotifications(handler);
  },
};
