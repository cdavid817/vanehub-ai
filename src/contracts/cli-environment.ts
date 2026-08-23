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

/**
 * Why an operation could not be verified after the fact.
 *
 * "We did not look" and "we looked and could not tell" both leave a stale snapshot, but they lead
 * to different advice, so they are separate codes.
 */
export type CliVerificationWarning =
  | "detection-skipped-while-busy"
  | "detection-failed"
  | "target-version-not-observed";

export const CLI_VERIFICATION_WARNINGS: readonly CliVerificationWarning[] = [
  "detection-skipped-while-busy",
  "detection-failed",
  "target-version-not-observed",
] as const;

/**
 * How the external process ended.
 *
 * Distinct from the outcome: `exited` with code 0 says the command reported success, while the
 * outcome says whether the machine was verified to match. Collapsing the two is how a successful
 * command with a failed verification gets shown as a clean success.
 */
export type CliOperationTermination =
  | "not-started"
  | "exited"
  | "exited-without-code"
  | "timed-out"
  | "cancelled";

export const CLI_OPERATION_TERMINATIONS: readonly CliOperationTermination[] = [
  "not-started",
  "exited",
  "exited-without-code",
  "timed-out",
  "cancelled",
] as const;

/**
 * Terminal result of a CLI lifecycle operation, carried on `OperationTask.result`.
 *
 * Every field is an identifier, a version, or one of the unions above. No path, credential, or
 * fragment of process output is representable here -- process output belongs on the operation's
 * log, where it has already been bounded and redacted.
 */
export interface CliMutationResult {
  operationId: string;
  agentId: string | null;
  sourceId: string | null;
  action: string | null;
  /** The version the plan aimed at. */
  targetVersion: string | null;
  /** The version detection actually saw afterwards, when it produced one. */
  observedVersion: string | null;
  phase: CliOperationPhase;
  termination: CliOperationTermination;
  /** Present only for `exited`; a timeout or cancellation reports no code rather than a made-up one. */
  exitCode: number | null;
  elapsedMs: number;
  outcome: CliMutationOutcome | null;
  warnings: readonly CliVerificationWarning[];
  outputTruncated: boolean;
  /** Whether the user should be told something beyond "done". */
  warning: boolean;
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

/**
 * Why a tool the user asked to upgrade is not in a batch, or did not run inside one.
 *
 * Mirrors `CliBulkSkipReason::as_str`. Stable codes rather than sentences, so the UI localizes
 * them and a test can assert on them.
 */
export type CliBulkSkipReason =
  | "already-current"
  | "detect-only-source"
  | "catalog-unavailable"
  | "needs-auth"
  | "broken"
  | "not-installed"
  | "unsupported-action"
  | "unordered-versions"
  | "source-ownership-unproven"
  | "plan-stale"
  | "plan-expired"
  | "plan-consumed"
  | "operation-conflict"
  | "installation-conflict";

export const CLI_BULK_SKIP_REASONS: readonly CliBulkSkipReason[] = [
  "already-current",
  "detect-only-source",
  "catalog-unavailable",
  "needs-auth",
  "broken",
  "not-installed",
  "unsupported-action",
  "unordered-versions",
  "source-ownership-unproven",
  "plan-stale",
  "plan-expired",
  "plan-consumed",
  "operation-conflict",
  "installation-conflict",
] as const;

/**
 * What became of one tool in a batch.
 *
 * A discriminated union, not one string vocabulary. An item either ran and produced one of the
 * five mutation outcomes, or it did not run and has a stable reason. The placeholder this replaces
 * said a process had started and nothing about whether the machine changed.
 */
export type CliBulkItemResult =
  | {
      status: "completed";
      agentId: string;
      planId: string | null;
      sourceId: string | null;
      targetVersion: string | null;
      outcome: CliMutationOutcome;
      reason: null;
    }
  | {
      status: "skipped";
      agentId: string;
      planId: string | null;
      sourceId: string | null;
      targetVersion: string | null;
      outcome: null;
      reason: CliBulkSkipReason;
    };

/** Terminal result of a bulk execution, carried on `OperationTask.result`. */
export interface CliBulkExecutionResult {
  /** Every tool the batch knew about, including the ones its plan already excluded. */
  items: readonly CliBulkItemResult[];
}
