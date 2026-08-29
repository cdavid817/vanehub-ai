export const activityEventCodes = [
  "run_started", "run_completed", "run_failed",
  "stage_started", "stage_completed", "stage_failed",
  "evidence_ready", "seed_ready", "assessment_completed", "assessment_needs_review",
  "generation_started", "generation_completed", "generation_failed", "dossier_completed",
  "curator_queued", "curator_approved", "curator_rejected", "curator_deferred",
  "overlay_previewed", "overlay_applied", "overlay_reverted",
  "automatic_eligible", "automatic_applied", "automatic_blocked",
  "probation_started", "probation_passed", "probation_regressed",
  "breaker_opened", "breaker_closed", "skill_created",
  "recovery_completed", "reconciliation_failed", "retention_applied", "source_purged",
] as const;

export type ActivityEventCode = (typeof activityEventCodes)[number];

export const activityStatuses = [
  "pending", "running", "succeeded", "failed", "blocked", "cancelled", "superseded",
] as const;

export type ActivityStatus = (typeof activityStatuses)[number];

export const activitySeverities = ["info", "warning", "error", "critical"] as const;
export type ActivitySeverity = (typeof activitySeverities)[number];

export const activityReasonCodes = [
  "started", "completed", "partial", "failed", "cancelled", "budget_exhausted",
  "evidence_ready", "seed_ready", "review_required", "policy_blocked",
  "validation_failed", "application_failed", "regression_detected", "breaker_opened",
  "integrity_failed", "security_blocked", "recovered", "retention_applied", "source_purged",
] as const;

export type ActivityReasonCode = (typeof activityReasonCodes)[number];

export const activityPayloadSchemas = [
  "status_card", "stage_timeline", "check_summary", "metric_summary",
  "navigation_list", "supersession_notice",
] as const;

export type ActivityPayloadSchema = (typeof activityPayloadSchemas)[number];

export type ActivityLabelCode = "outcome" | "current_stage" | "governance_decision"
  | "application_status" | "retention_outcome";
export type ActivityValueCode = "started" | "pending" | "running" | "ready" | "succeeded"
  | "completed" | "failed" | "blocked" | "cancelled" | "superseded" | "eligible"
  | "ineligible" | "approved" | "rejected" | "deferred" | "applied" | "reverted"
  | "healthy" | "regressed" | "open" | "closed" | "created" | "purged";
export type ActivityStageCode = "recover" | "maintain_evidence" | "build_seeds" | "assess"
  | "route_governance" | "evaluate_auto_apply" | "project_results" | "notify";
export type ActivityMetricCode = "candidate_count" | "evidence_count" | "passed_check_count"
  | "failed_check_count" | "review_check_count" | "applied_count" | "rejected_count"
  | "purged_count" | "duration_ms";
export type ActivityNavigationKind = "run" | "evidence" | "assessment" | "dossier"
  | "generation_job" | "curator_candidate" | "overlay_history" | "skill" | "probation"
  | "breaker";

export interface ActivityNavigation {
  kind: ActivityNavigationKind;
  stableId: string;
  childId?: string | null;
}

export type ActivityPayload =
  | { schema: "status_card"; labelCode: ActivityLabelCode; valueCode: ActivityValueCode }
  | { schema: "stage_timeline"; stages: Array<{ code: ActivityStageCode; status: ActivityStatus }> }
  | { schema: "check_summary"; passed: number; failed: number; review: number }
  | { schema: "metric_summary"; metrics: Partial<Record<ActivityMetricCode, number>> }
  | { schema: "navigation_list"; links: ActivityNavigation[] }
  | { schema: "supersession_notice"; priorEventId: string };
