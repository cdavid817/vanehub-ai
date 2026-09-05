import type { SkillCuratorService } from "../services/skill-curator-service";
import type { CuratorResult } from "../types/skill-curator";

const unavailable = async (): Promise<CuratorResult<never>> => ({
  ok: false,
  error: {
    code: "storage_unavailable",
    message: "skill_curator_unavailable",
  },
});

// Keeps the aggregate boundary fail-closed while each runtime adapter is wired.
export const unavailableSkillCuratorClient: SkillCuratorService = {
  querySkillCuratorQueue: unavailable,
  getSkillCuratorCandidate: unavailable,
  querySkillCuratorAudit: unavailable,
  getSkillCuratorPolicy: unavailable,
  updateSkillCuratorPolicy: unavailable,
  saveSkillCuratorDraft: unavailable,
  previewSkillCuratorCandidate: unavailable,
  approveSkillCuratorCandidate: unavailable,
  rejectSkillCuratorCandidate: unavailable,
  deferSkillCuratorCandidate: unavailable,
  resumeSkillCuratorCandidate: unavailable,
  retrySkillCuratorApplication: unavailable,
  async subscribeSkillCuratorNotifications() {
    return () => undefined;
  },
};
