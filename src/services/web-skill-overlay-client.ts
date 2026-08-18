import type { SkillOverlayService } from "./skill-service";
import { createWebSkillOverlayRuntime } from "./web-skill-overlay-runtime";
import { overlayError, webOverlayHash } from "./web-skill-overlay-support";
import { normalizeWebPath } from "./web-skill-location";
import { buildSkillContent, listWebSkills } from "./web-skill-state";

// The resolver reads the Skill catalogue through the state module's accessor. Closing over an
// imported `let` instead would be the divergent-mock-world hazard the state modules exist to avoid.
const webSkillOverlayRuntime = createWebSkillOverlayRuntime((target) => {
  const workspacePath = target.scope === "project" && target.workspacePath
    ? normalizeWebPath(target.workspacePath, "Workspace path")
    : null;
  const candidates = listWebSkills().filter((skill) =>
    skill.id === target.skillId
      && (skill.scope === "global" || (workspacePath != null && skill.workspacePath === workspacePath)),
  );
  const skill = candidates.find((candidate) => candidate.scope === "workspace") ?? candidates[0];
  if (!skill) throw overlayError("notFound", "skill-not-found", `Skill not found: ${target.skillId}`);
  const instructions = buildSkillContent(skill);
  return {
    skillId: skill.id,
    layer: skill.layer,
    instructions,
    instructionHash: webOverlayHash(instructions),
    packageHash: skill.contentHash,
    pinned: false,
  };
});

export const webSkillOverlayClient: SkillOverlayService = {
  async getSkillOverlaySummary(input) {
    return webSkillOverlayRuntime.getSummary(input);
  },

  async getSkillOverlayDetail(input) {
    return webSkillOverlayRuntime.getDetail(input);
  },

  async previewSkillOverlay(input) {
    return webSkillOverlayRuntime.preview(input);
  },

  async getSkillOverlayHistory(input) {
    return webSkillOverlayRuntime.getHistory(input);
  },

  async createSkillOverlayPatch(input) {
    return webSkillOverlayRuntime.createPatch(input);
  },

  async createSkillOverlayGuidance(input) {
    return webSkillOverlayRuntime.createGuidance(input);
  },

  async addSkillOverlayFile(input) {
    return webSkillOverlayRuntime.addFile(input);
  },

  async replaceSkillOverlayFile(input) {
    return webSkillOverlayRuntime.replaceFile(input);
  },

  async importSkillOverlay(input) {
    return webSkillOverlayRuntime.importOverlay(input);
  },

  async promoteSkillOverlay(input) {
    return webSkillOverlayRuntime.promote(input);
  },

  async disableSkillOverlayMutation(input) {
    return webSkillOverlayRuntime.disable(input);
  },

  async revertSkillOverlayMutation(input) {
    return webSkillOverlayRuntime.revert(input);
  },

  async previewSkillOverlayReconciliation(input) {
    return webSkillOverlayRuntime.previewReconciliation(input);
  },

  async reconcileSkillOverlay(input) {
    return webSkillOverlayRuntime.reconcile(input);
  },
};
