import { invoke } from "@tauri-apps/api/core";
import type { SkillAssessmentService } from "./skill-assessment-service";

export const tauriSkillAssessmentClient: SkillAssessmentService = {
  querySkillEvolutionAssessments(input) {
    return invoke("query_skill_evolution_assessments", { input });
  },
  getSkillEvolutionAssessment(attemptId) {
    return invoke("get_skill_evolution_assessment", { attemptId });
  },
  getSkillEvolutionAssessmentPolicy() {
    return invoke("get_skill_evolution_assessment_policy");
  },
  updateSkillEvolutionAssessmentConsent(input) {
    return invoke("update_skill_evolution_assessment_consent", { input });
  },
  scheduleSkillEvolutionReassessment(input) {
    return invoke("schedule_skill_evolution_reassessment", { input });
  },
};
