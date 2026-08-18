import type {
  Skill,
  SkillAgentMountPath,
  SkillImportInput,
  SkillListResult,
  SkillLoadInput,
  SkillLoadOutcome,
  SkillMountMigrationReport,
  SkillMutationInput,
  SkillOverview,
  SkillPreview,
  SkillResourceReadInput,
  SkillResourceReadOutcome,
  SkillScopeInput,
  SkillUpdateInput,
} from "../types/skill";
import type {
  SkillOverlayDetail,
  SkillOverlayFileInput,
  SkillOverlayGuidanceInput,
  SkillOverlayHistoryInput,
  SkillOverlayHistoryPage,
  SkillOverlayImportInput,
  SkillOverlayImportReview,
  SkillOverlayMutationOutcome,
  SkillOverlayMutationStateInput,
  SkillOverlayPatchInput,
  SkillOverlayPreview,
  SkillOverlayPreviewInput,
  SkillOverlayPromotionInput,
  SkillOverlaySummary,
  SkillOverlayTargetInput,
} from "../types/skill-overlay";
import type {
  SkillOverlayReconciliationInput,
  SkillOverlayReconciliationPreview,
} from "../types/skill-overlay-reconciliation";

export interface SkillCatalogService {
  listSkills(input: SkillScopeInput): Promise<SkillListResult>;
  getSkillOverview(input: SkillScopeInput): Promise<SkillOverview>;
  listSkillMountPaths(): Promise<SkillAgentMountPath[]>;
  updateSkillMountPath(agentId: string, mountPath: string): Promise<SkillMountMigrationReport>;
  createSkill(input: SkillMutationInput): Promise<Skill>;
  updateSkill(skillId: string, input: SkillUpdateInput): Promise<Skill>;
  deleteSkill(skillId: string, input: SkillScopeInput): Promise<void>;
  restoreBuiltinSkill(skillId: string): Promise<Skill>;
  importSkill(input: SkillImportInput): Promise<Skill>;
}

export interface SkillBindingService {
  setSkillEnabled(skillId: string, input: SkillScopeInput, enabled: boolean): Promise<Skill>;
  setSkillAgentBindings(skillId: string, input: SkillScopeInput, agentIds: string[]): Promise<Skill>;
  bindSkillToCliAgent(skillId: string, input: SkillScopeInput, agentId: string): Promise<Skill>;
  unbindSkillFromCliAgent(skillId: string, input: SkillScopeInput, agentId: string): Promise<Skill>;
  bindSkillToApiAgent(skillId: string, input: SkillScopeInput, agentId: string): Promise<void>;
  unbindSkillFromApiAgent(skillId: string, input: SkillScopeInput, agentId: string): Promise<void>;
  listSkillApiAgentBindings(skillId: string, input: SkillScopeInput): Promise<string[]>;
  previewSkill(skillId: string, input: SkillScopeInput): Promise<SkillPreview>;
  loadSkill(input: SkillLoadInput): Promise<SkillLoadOutcome>;
  readSkillResource(input: SkillResourceReadInput): Promise<SkillResourceReadOutcome>;
}

export interface SkillOverlayService {
  getSkillOverlaySummary(input: SkillOverlayTargetInput): Promise<SkillOverlaySummary>;
  getSkillOverlayDetail(input: SkillOverlayTargetInput): Promise<SkillOverlayDetail>;
  previewSkillOverlay(input: SkillOverlayPreviewInput): Promise<SkillOverlayPreview>;
  getSkillOverlayHistory(input: SkillOverlayHistoryInput): Promise<SkillOverlayHistoryPage>;
  createSkillOverlayPatch(input: SkillOverlayPatchInput): Promise<SkillOverlayMutationOutcome>;
  createSkillOverlayGuidance(input: SkillOverlayGuidanceInput): Promise<SkillOverlayMutationOutcome>;
  addSkillOverlayFile(input: SkillOverlayFileInput): Promise<SkillOverlayMutationOutcome>;
  replaceSkillOverlayFile(input: SkillOverlayFileInput): Promise<SkillOverlayMutationOutcome>;
  importSkillOverlay(input: SkillOverlayImportInput): Promise<SkillOverlayImportReview>;
  promoteSkillOverlay(input: SkillOverlayPromotionInput): Promise<SkillOverlayMutationOutcome>;
  disableSkillOverlayMutation(
    input: SkillOverlayMutationStateInput,
  ): Promise<SkillOverlayMutationOutcome>;
  revertSkillOverlayMutation(
    input: SkillOverlayMutationStateInput,
  ): Promise<SkillOverlayMutationOutcome>;
  previewSkillOverlayReconciliation(
    input: SkillOverlayReconciliationInput,
  ): Promise<SkillOverlayReconciliationPreview>;
  reconcileSkillOverlay(
    input: SkillOverlayReconciliationInput,
  ): Promise<SkillOverlayMutationOutcome>;
}
