import {
  activityStatuses, type ActivityMetricCode, type ActivityNavigation, type ActivityPayload,
} from "./activity-contracts";

const labels = new Set(["outcome", "current_stage", "governance_decision", "application_status", "retention_outcome"]);
const values = new Set([
  "started", "pending", "running", "ready", "succeeded", "completed", "failed", "blocked",
  "cancelled", "superseded", "eligible", "ineligible", "approved", "rejected", "deferred",
  "applied", "reverted", "healthy", "regressed", "open", "closed", "created", "purged",
]);
const stages = new Set([
  "recover", "maintain_evidence", "build_seeds", "assess", "route_governance",
  "evaluate_auto_apply", "project_results", "notify",
]);
const metrics = new Set<ActivityMetricCode>([
  "candidate_count", "evidence_count", "passed_check_count", "failed_check_count",
  "review_check_count", "applied_count", "rejected_count", "purged_count", "duration_ms",
]);
const navigationKinds = new Set([
  "run", "evidence", "assessment", "dossier", "generation_job", "curator_candidate",
  "overlay_history", "skill", "probation", "breaker",
]);
const statuses = new Set<string>(activityStatuses);
const MAX_COLLECTION_ITEMS = 24;

export function decodeActivityPayload(input: unknown): ActivityPayload | null {
  if (!isRecord(input) || typeof input.schema !== "string") return null;
  switch (input.schema) {
    case "status_card":
      return exactKeys(input, ["schema", "labelCode", "valueCode"])
        && isMember(input.labelCode, labels) && isMember(input.valueCode, values)
        ? input as ActivityPayload
        : null;
    case "stage_timeline":
      return decodeStageTimeline(input);
    case "check_summary":
      return exactKeys(input, ["schema", "passed", "failed", "review"])
        && isCount(input.passed) && isCount(input.failed) && isCount(input.review)
        ? input as ActivityPayload
        : null;
    case "metric_summary":
      return decodeMetricSummary(input);
    case "navigation_list":
      return decodeNavigationList(input);
    case "supersession_notice":
      return exactKeys(input, ["schema", "priorEventId"]) && isSafeId(input.priorEventId)
        ? input as ActivityPayload
        : null;
    default:
      return null;
  }
}

function decodeStageTimeline(input: Record<string, unknown>): ActivityPayload | null {
  if (!exactKeys(input, ["schema", "stages"]) || !Array.isArray(input.stages)
    || input.stages.length > MAX_COLLECTION_ITEMS) return null;
  if (!input.stages.every((stage) => isRecord(stage)
    && exactKeys(stage, ["code", "status"])
    && isMember(stage.code, stages)
    && isMember(stage.status, statuses))) return null;
  return input as ActivityPayload;
}

function decodeMetricSummary(input: Record<string, unknown>): ActivityPayload | null {
  if (!exactKeys(input, ["schema", "metrics"]) || !isRecord(input.metrics)) return null;
  const entries = Object.entries(input.metrics);
  if (entries.length > MAX_COLLECTION_ITEMS
    || entries.some(([code, value]) => !metrics.has(code as ActivityMetricCode) || !isSafeNumber(value))) {
    return null;
  }
  return input as ActivityPayload;
}

function decodeNavigationList(input: Record<string, unknown>): ActivityPayload | null {
  if (!exactKeys(input, ["schema", "links"]) || !Array.isArray(input.links)
    || input.links.length > 16) return null;
  const links: ActivityNavigation[] = [];
  for (const link of input.links) {
    const decoded = decodeActivityNavigation(link);
    if (!decoded) return null;
    links.push(decoded);
  }
  return { schema: "navigation_list", links };
}

export function decodeActivityNavigation(input: unknown): ActivityNavigation | null {
  if (!isRecord(input) || !exactOptionalKeys(input, ["kind", "stableId"], ["childId"])
    || !isMember(input.kind, navigationKinds) || !isSafeId(input.stableId)
    || !(input.childId === undefined || input.childId === null || isSafeId(input.childId))) {
    return null;
  }
  return {
    kind: input.kind as ActivityNavigation["kind"],
    stableId: input.stableId,
    ...(input.childId === undefined ? {} : { childId: input.childId }),
  };
}

function isRecord(value: unknown): value is Record<string, unknown> {
  return typeof value === "object" && value !== null && !Array.isArray(value);
}

function exactKeys(value: Record<string, unknown>, required: string[]): boolean {
  return exactOptionalKeys(value, required, []);
}

function exactOptionalKeys(
  value: Record<string, unknown>,
  required: string[],
  optional: string[],
): boolean {
  const keys = Object.keys(value);
  return required.every((key) => keys.includes(key))
    && keys.every((key) => required.includes(key) || optional.includes(key));
}

function isMember(value: unknown, registry: ReadonlySet<string>): value is string {
  return typeof value === "string" && registry.has(value);
}

function isCount(value: unknown): value is number {
  return typeof value === "number" && Number.isSafeInteger(value) && value >= 0;
}

function isSafeNumber(value: unknown): value is number {
  return typeof value === "number" && Number.isSafeInteger(value);
}

function isSafeId(value: unknown): value is string {
  return typeof value === "string" && value.length > 0 && value.length <= 160
    && /^[\p{L}\p{N}\-_.:@]+$/u.test(value);
}
