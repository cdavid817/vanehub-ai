import {
  Activity, AlertTriangle, Ban, BellRing, CheckCircle2, CircleDashed, ClipboardCheck,
  DatabaseZap, FileClock, Gauge, GitCommitHorizontal, History, ListChecks, type LucideIcon,
  Network, PackageCheck, RefreshCw, RotateCcw, ShieldAlert, ShieldCheck, Sparkles,
  TimerReset, Wrench,
} from "lucide-react";
import type {
  ActivityEventCode, ActivityPayloadSchema, ActivityReasonCode, ActivitySeverity, ActivityStatus,
} from "./activity-contracts";

export interface ActivityPresentationEntry {
  titleKey: `systemActivity.${string}`;
  accessibleLabelKey: `systemActivity.${string}`;
  icon: LucideIcon;
}

export interface ActivityPayloadPresentation {
  renderer: "status-card" | "stage-timeline" | "check-summary" | "metric-summary"
    | "navigation-list" | "supersession-notice";
  accessibleLabelKey: `systemActivity.${string}`;
}

function entry(code: string, icon: LucideIcon): ActivityPresentationEntry {
  return {
    titleKey: `systemActivity.event.${code}.title`,
    accessibleLabelKey: `systemActivity.event.${code}.title`,
    icon,
  };
}

export const activityEventPresentation = {
  run_started: entry("runStarted", Activity),
  run_completed: entry("runCompleted", CheckCircle2),
  run_failed: entry("runFailed", AlertTriangle),
  stage_started: entry("stageStarted", CircleDashed),
  stage_completed: entry("stageCompleted", CheckCircle2),
  stage_failed: entry("stageFailed", AlertTriangle),
  evidence_ready: entry("evidenceReady", DatabaseZap),
  seed_ready: entry("seedReady", Sparkles),
  assessment_completed: entry("assessmentCompleted", ClipboardCheck),
  assessment_needs_review: entry("assessmentNeedsReview", BellRing),
  generation_started: entry("generationStarted", Wrench),
  generation_completed: entry("generationCompleted", PackageCheck),
  generation_failed: entry("generationFailed", AlertTriangle),
  dossier_completed: entry("dossierCompleted", ListChecks),
  curator_queued: entry("curatorQueued", FileClock),
  curator_approved: entry("curatorApproved", ShieldCheck),
  curator_rejected: entry("curatorRejected", Ban),
  curator_deferred: entry("curatorDeferred", TimerReset),
  overlay_previewed: entry("overlayPreviewed", GitCommitHorizontal),
  overlay_applied: entry("overlayApplied", CheckCircle2),
  overlay_reverted: entry("overlayReverted", RotateCcw),
  automatic_eligible: entry("automaticEligible", Gauge),
  automatic_applied: entry("automaticApplied", PackageCheck),
  automatic_blocked: entry("automaticBlocked", ShieldAlert),
  probation_started: entry("probationStarted", TimerReset),
  probation_passed: entry("probationPassed", ShieldCheck),
  probation_regressed: entry("probationRegressed", AlertTriangle),
  breaker_opened: entry("breakerOpened", ShieldAlert),
  breaker_closed: entry("breakerClosed", ShieldCheck),
  skill_created: entry("skillCreated", Sparkles),
  recovery_completed: entry("recoveryCompleted", RefreshCw),
  reconciliation_failed: entry("reconciliationFailed", AlertTriangle),
  retention_applied: entry("retentionApplied", History),
  source_purged: entry("sourcePurged", DatabaseZap),
} satisfies Record<ActivityEventCode, ActivityPresentationEntry>;

function semanticEntry(group: string, code: string, icon: LucideIcon): ActivityPresentationEntry {
  return {
    titleKey: `systemActivity.${group}.${code}.title`,
    accessibleLabelKey: `systemActivity.${group}.${code}.title`,
    icon,
  };
}

export const activityStatusPresentation = {
  pending: semanticEntry("status", "pending", CircleDashed),
  running: semanticEntry("status", "running", Activity),
  succeeded: semanticEntry("status", "succeeded", CheckCircle2),
  failed: semanticEntry("status", "failed", AlertTriangle),
  blocked: semanticEntry("status", "blocked", Ban),
  cancelled: semanticEntry("status", "cancelled", Ban),
  superseded: semanticEntry("status", "superseded", History),
} satisfies Record<ActivityStatus, ActivityPresentationEntry>;

export const activitySeverityPresentation = {
  info: semanticEntry("severity", "info", Activity),
  warning: semanticEntry("severity", "warning", AlertTriangle),
  error: semanticEntry("severity", "error", ShieldAlert),
  critical: semanticEntry("severity", "critical", ShieldAlert),
} satisfies Record<ActivitySeverity, ActivityPresentationEntry>;

const reasonIcon: Record<ActivityReasonCode, LucideIcon> = {
  started: Activity, completed: CheckCircle2, partial: Gauge, failed: AlertTriangle,
  cancelled: Ban, budget_exhausted: TimerReset, evidence_ready: DatabaseZap,
  seed_ready: Sparkles, review_required: BellRing, policy_blocked: ShieldAlert,
  validation_failed: AlertTriangle, application_failed: AlertTriangle,
  regression_detected: AlertTriangle, breaker_opened: ShieldAlert,
  integrity_failed: ShieldAlert, security_blocked: ShieldAlert, recovered: RefreshCw,
  retention_applied: History, source_purged: DatabaseZap,
};

export const activityReasonPresentation = Object.fromEntries(
  Object.entries(reasonIcon).map(([code, icon]) => [code, semanticEntry("reason", code, icon)]),
) as Record<ActivityReasonCode, ActivityPresentationEntry>;

export const activityPayloadPresentation = {
  status_card: { renderer: "status-card", accessibleLabelKey: "systemActivity.payload.statusCard.label" },
  stage_timeline: { renderer: "stage-timeline", accessibleLabelKey: "systemActivity.payload.stageTimeline.label" },
  check_summary: { renderer: "check-summary", accessibleLabelKey: "systemActivity.payload.checkSummary.label" },
  metric_summary: { renderer: "metric-summary", accessibleLabelKey: "systemActivity.payload.metricSummary.label" },
  navigation_list: { renderer: "navigation-list", accessibleLabelKey: "systemActivity.payload.navigationList.label" },
  supersession_notice: { renderer: "supersession-notice", accessibleLabelKey: "systemActivity.payload.supersessionNotice.label" },
} satisfies Record<ActivityPayloadSchema, ActivityPayloadPresentation>;

export const activityPayloadIcons: Record<ActivityPayloadSchema, LucideIcon> = {
  status_card: Gauge,
  stage_timeline: Network,
  check_summary: ListChecks,
  metric_summary: Gauge,
  navigation_list: Network,
  supersession_notice: History,
};
