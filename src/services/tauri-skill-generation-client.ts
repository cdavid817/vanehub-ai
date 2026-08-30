import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import type { GenerationNotificationEvent } from "./skill-generation-service";
import type { SkillGenerationService } from "./skill-generation-service";

export const tauriSkillGenerationClient: SkillGenerationService = {
  getGenerationPolicy(workspaceId) {
    return invoke("get_skill_evolution_generation_policy", { workspaceId });
  },
  updateGenerationPolicy(input) {
    return invoke("update_skill_evolution_generation_policy", { input });
  },
  listGenerationJobs(input) {
    return invoke("query_skill_evolution_generation_jobs", { input });
  },
  getGenerationJob(jobId) {
    return invoke("get_skill_evolution_generation_job", { jobId });
  },
  cancelGenerationJob(jobId) {
    return invoke("cancel_skill_evolution_generation_job", { jobId });
  },
  regenerateGenerationJob(input) {
    return invoke("regenerate_skill_evolution_generation_job", { input });
  },
  getGenerationDossierSection(dossierId, ordinal, cursor, limit) {
    return invoke("get_skill_evolution_generation_dossier_section", {
      input: { dossierId, ordinal, cursor, limit },
    });
  },
  getGenerationProvenance(jobId) {
    return invoke("get_skill_evolution_generation_provenance", { jobId });
  },
  listGenerationQuarantine(input) {
    return invoke("query_skill_evolution_generation_quarantine", { input });
  },
  exportGenerationDossier(input) {
    return invoke("export_skill_evolution_generation_dossier", { input });
  },
  handoffGenerationPackage(jobId) {
    return invoke("handoff_skill_evolution_generation_package", { jobId });
  },
  async subscribeGenerationNotifications(handler) {
    return listen<GenerationNotificationEvent>("skill-generation:notification", ({ payload }) => handler(payload));
  },
};
