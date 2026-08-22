/**
 * Normalized source-aware CLI environment contract.
 *
 * Mirrors `src-tauri/src/contexts/tooling/cli/domain/`. Every union here has a matching Rust
 * `as_str` covered by a test on that side, so a rename fails both suites rather than silently
 * producing a value the other end does not understand.
 *
 * Frontend code renders these. It does not compare versions, derive upgrade versus downgrade, or
 * decide whether a source is manageable -- the backend already did.
 */

/**
 * How a completed machine mutation actually ended.
 *
 * `applied-unverified` and `changed-but-failed` exist because a package manager is an external
 * effect: it cannot be rolled back by writing an older row, so a command that succeeded while
 * verification failed must not be reported as "nothing happened".
 */
export type CliMutationOutcome =
  | "verified"
  | "applied-unverified"
  | "changed-but-failed"
  | "no-change-failed"
  | "cancelled";

/** Every `CliMutationOutcome`, for exhaustiveness checks and contract drift tests. */
export const CLI_MUTATION_OUTCOMES: readonly CliMutationOutcome[] = [
  "verified",
  "applied-unverified",
  "changed-but-failed",
  "no-change-failed",
  "cancelled",
] as const;

/** Outcomes that mean the host may have changed, so cached values are no longer authoritative. */
export const CLI_OUTCOMES_THAT_MAY_HAVE_CHANGED_THE_MACHINE: readonly CliMutationOutcome[] = [
  "verified",
  "applied-unverified",
  "changed-but-failed",
] as const;

/**
 * Why a plan cannot run. The frontend maps these to localized text; it never parses an error
 * string.
 */
export type CliPlanRejectionCode =
  | "plan-revision-mismatch"
  | "plan-expired"
  | "plan-consumed"
  | "plan-stale";

export const CLI_PLAN_REJECTION_CODES: readonly CliPlanRejectionCode[] = [
  "plan-revision-mismatch",
  "plan-expired",
  "plan-consumed",
  "plan-stale",
] as const;

/** Terminal result of a CLI lifecycle operation, carried on `OperationTask.result`. */
export interface CliMutationResult {
  agentId: string;
  sourceId: string;
  action: string;
  targetVersion: string | null;
  outcome: CliMutationOutcome;
  /** Present when the outcome is not `verified`; a localized hint about what to do next. */
  warningCode: string | null;
}

/** Descriptive stages a CLI operation reports through `OperationTask.phase`. */
export type CliOperationPhase =
  | "preflight"
  | "resolving-source"
  | "querying-catalog"
  | "planning"
  | "downloading"
  | "mutating"
  | "verifying-executable"
  | "refreshing-environment"
  | "running-doctor"
  | "completed";

export const CLI_OPERATION_PHASES: readonly CliOperationPhase[] = [
  "preflight",
  "resolving-source",
  "querying-catalog",
  "planning",
  "downloading",
  "mutating",
  "verifying-executable",
  "refreshing-environment",
  "running-doctor",
  "completed",
] as const;
