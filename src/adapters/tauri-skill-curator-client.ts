import { invoke } from "@tauri-apps/api/core";
import type { SkillCuratorService } from "../services/skill-curator-service";
import type {
  CuratorCandidateState,
  CuratorError,
  CuratorResult,
  CuratorSafeState,
} from "../types/skill-curator";
import { subscribeTauriSkillCuratorNotifications } from "./tauri-skill-curator-notifications";

const errorCodes = new Set<CuratorError["code"]>([
  "not_found",
  "invalid_input",
  "unsafe_content",
  "not_approvable",
  "stale_conflict",
  "preview_expired",
  "pinned",
  "application_failed",
  "storage_unavailable",
]);

const candidateStates = new Set<CuratorCandidateState>([
  "pending",
  "awaiting_draft",
  "ready_for_review",
  "deferred",
  "rejected",
  "applying",
  "applied",
  "apply_failed",
  "superseded",
]);

function isRecord(value: unknown): value is Record<string, unknown> {
  return typeof value === "object" && value !== null;
}

function parseNativeError(error: unknown): unknown {
  if (typeof error !== "string") return error;
  try {
    return JSON.parse(error) as unknown;
  } catch {
    return error;
  }
}

function readSafeState(value: unknown): CuratorSafeState | undefined {
  if (!isRecord(value)) return undefined;
  const { candidateId, revision, state, witnessHash, policyWitnessHash, currentPreviewId } = value;
  if (
    typeof candidateId !== "string"
    || typeof revision !== "number"
    || !Number.isSafeInteger(revision)
    || revision < 0
    || typeof state !== "string"
    || !candidateStates.has(state as CuratorCandidateState)
    || typeof witnessHash !== "string"
    || typeof policyWitnessHash !== "string"
    || (currentPreviewId !== undefined && typeof currentPreviewId !== "string")
  ) return undefined;
  return {
    candidateId,
    revision,
    state: state as CuratorCandidateState,
    witnessHash,
    policyWitnessHash,
    ...(currentPreviewId === undefined ? {} : { currentPreviewId }),
  };
}

function normalizeNativeError(error: unknown): CuratorError {
  const value = parseNativeError(error);
  if (!isRecord(value) || typeof value.code !== "string" || !errorCodes.has(value.code as CuratorError["code"])) {
    return { code: "storage_unavailable", message: "skill_curator_native_failure" };
  }
  const current = readSafeState(value.current);
  return {
    code: value.code as CuratorError["code"],
    message: typeof value.message === "string" ? value.message : value.code,
    ...(current === undefined ? {} : { current }),
    ...(typeof value.field === "string" ? { field: value.field } : {}),
    ...(typeof value.reasonCode === "string" ? { reasonCode: value.reasonCode } : {}),
  };
}

async function invokeCurator<T>(command: string, args?: Record<string, unknown>): Promise<CuratorResult<T>> {
  try {
    return { ok: true, value: await invoke<T>(command, args) };
  } catch (error: unknown) {
    return { ok: false, error: normalizeNativeError(error) };
  }
}

export const tauriSkillCuratorClient: SkillCuratorService = {
  querySkillCuratorQueue(input) {
    return invokeCurator("query_skill_curator_queue", { input });
  },
  getSkillCuratorCandidate(candidateId) {
    return invokeCurator("get_skill_curator_candidate", { candidateId });
  },
  querySkillCuratorAudit(candidateId, cursor) {
    return invokeCurator("query_skill_curator_audit", { input: { candidateId, cursor } });
  },
  getSkillCuratorPolicy(workspaceId) {
    return invokeCurator("get_skill_curator_policy", { workspaceId });
  },
  updateSkillCuratorPolicy(input) {
    return invokeCurator("update_skill_curator_policy", { input });
  },
  saveSkillCuratorDraft(input) {
    return invokeCurator("save_skill_curator_draft", { input });
  },
  previewSkillCuratorCandidate(input) {
    return invokeCurator("preview_skill_curator_candidate", { input });
  },
  approveSkillCuratorCandidate(input) {
    return invokeCurator("approve_skill_curator_candidate", { input });
  },
  rejectSkillCuratorCandidate(input) {
    return invokeCurator("reject_skill_curator_candidate", { input });
  },
  deferSkillCuratorCandidate(input) {
    return invokeCurator("defer_skill_curator_candidate", { input });
  },
  resumeSkillCuratorCandidate(input) {
    return invokeCurator("resume_skill_curator_candidate", { input });
  },
  retrySkillCuratorApplication(input) {
    return invokeCurator("retry_skill_curator_application", { input });
  },
  subscribeSkillCuratorNotifications: subscribeTauriSkillCuratorNotifications,
};
