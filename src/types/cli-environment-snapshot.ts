/**
 * The source-aware CLI environment snapshot and action-plan shapes.
 *
 * Mirrors `src-tauri/src/commands/tooling/cli_environment/dto.rs` field for field. Every union
 * value is a string the Rust side emits from an `as_str` that a drift test on that side pins, so a
 * rename fails one suite or the other rather than silently producing a value neither end
 * understands.
 *
 * Components render these. They do not compare versions, decide upgrade versus downgrade, derive
 * what a source can do, or build a command -- the backend already did all four, and doing them
 * again here is exactly the divergence this change removes.
 *
 * Split from `cli-environment.ts` to stay inside the 300-line file rule; that file owns the
 * lifecycle vocabularies, this one owns the record shapes.
 */

/** An executable found on this host, with everything known about where it came from. */
export interface CliInstallation {
  id: string;
  executablePath: string;
  canonicalPath: string | null;
  /**
   * Launcher aliases folded into this one installation -- `claude`, `claude.cmd`, `claude.ps1` on
   * Windows. Present so the UI can explain why three files on disk are one entry.
   */
  aliasPaths: readonly string[];
  /** A launcher whose target no longer exists. It is on PATH and it cannot run. */
  targetMissing: boolean;
  reportedVersion: string | null;
  sourceId: string | null;
  sourceKind: string;
  /** `unknown` < `inferred` < `verified`. A path heuristic is inferred, never verified. */
  sourceConfidence: string;
  /** Position in PATH. `null` means it was found in a known location, not on PATH. */
  pathPriority: number | null;
  environmentOrigin: string;
  executableStatus: string;
}

/**
 * A structured problem with how this tool is installed.
 *
 * `blocksMutation` and `blocksLaunch` are decided by the backend. A UI that re-derived them from
 * `kind` would disagree with the backend the first time a kind's severity changed.
 */
export interface CliConflict {
  kind: string;
  severity: string;
  installationIds: readonly string[];
  blocksMutation: boolean;
  blocksLaunch: boolean;
  /** The localization key. The frontend never parses `kind` or a message string. */
  reasonCode: string;
}

export interface CliSourceCapabilities {
  install: string;
  upgrade: string;
  downgrade: string;
  reinstall: string;
  uninstall: boolean;
  repair: string;
}

export interface CliSourceSummary {
  sourceId: string;
  kind: string;
  supportedOnThisPlatform: boolean;
  /** Present only when the source was reachable and reported one. */
  availableVersionCount: number | null;
  /**
   * This source's own list, newest first. A target selector reads it rather than rebuilding one,
   * and two sources' lists are never merged -- borrowing another source's catalog is the defect
   * this change removes.
   */
  availableVersions: readonly string[];
  capabilities: CliSourceCapabilities;
}

/**
 * An action the backend will accept for this tool, from this source.
 *
 * An action absent from this list is one the UI must not offer. `reasonCode` explains a withheld
 * action when the backend has something to say about it.
 */
export interface CliAllowedAction {
  action: string;
  sourceId: string;
  targetMode: string;
  /** What the action aims at when the user picks nothing. Never substituted for a chosen target. */
  defaultTarget: string | null;
  requiresTargetSelection: boolean;
  reasonCode: string | null;
}

export interface CliMutationSummary {
  outcome: string;
  sourceId: string;
  action: string;
  targetVersion: string | null;
  operationId: string;
  completedAt: string;
}

/** Everything known about one CLI tool's environment on this host. */
export interface CliEnvironmentSnapshot {
  schemaVersion: number;
  agentId: string;
  /**
   * Identity from the backend registry. Carried on the snapshot so no component keeps a second
   * copy of the tool catalog -- a frontend list of display names drifts the first time a tool is
   * renamed on the other side.
   */
  displayName: string;
  provider: string;
  /** Every name this executable may carry. Windows shims mean one CLI appears under several. */
  executableNames: readonly string[];
  scope: string;
  overallState: string;
  freshness: string;
  /**
   * Identifies the environment this was captured against. A plan is bound to it, so a snapshot
   * whose fingerprint no longer matches cannot authorize a mutation.
   */
  environmentFingerprint: string;
  installations: readonly CliInstallation[];
  /** What this host's PATH would run. `null` means nothing is on PATH -- a real answer. */
  pathSelectedInstallationId: string | null;
  /** What the backend would act on. Differs from the PATH-selected one when something is wrong. */
  recommendedInstallationId: string | null;
  discovery: string;
  executable: string;
  authentication: string;
  readiness: string;
  compatibility: string;
  update: string;
  conflicts: readonly CliConflict[];
  sources: readonly CliSourceSummary[];
  allowedActions: readonly CliAllowedAction[];
  lastMutation: CliMutationSummary | null;
  lastOperationId: string | null;
  checkedAt: string | null;
}

/** The argv a plan will run. Structured, never a shell string. */
export interface CliCommandPreview {
  program: string;
  args: readonly string[];
}

/**
 * A single-use, revisioned plan for one machine change.
 *
 * The user reviews this, then execution submits only its id and revision. There is no field here a
 * caller could rebuild a command from, which is what makes "the version reviewed is the version
 * that runs" structural rather than a convention.
 */
export interface CliActionPlan {
  id: string;
  /** Submitted back on execution. A plan revised in between is refused rather than run. */
  revision: number;
  agentId: string;
  action: string;
  sourceId: string;
  installationId: string | null;
  currentVersion: string | null;
  targetVersion: string | null;
  channel: string | null;
  commandPreview: CliCommandPreview;
  preconditions: readonly string[];
  warnings: readonly string[];
  requiresElevation: boolean;
  requiresNetwork: boolean;
  state: string;
  createdAt: string;
  expiresAt: string;
}

export interface CliBulkActionItem {
  agentId: string;
  planId: string;
  sourceId: string;
  currentVersion: string | null;
  targetVersion: string | null;
}

/** A tool left out of a batch, and why. A shorter list with no reason reads as "nothing to do". */
export interface CliBulkSkip {
  agentId: string;
  reason: string;
}

export interface CliBulkActionPlan {
  id: string;
  revision: number;
  items: readonly CliBulkActionItem[];
  skipped: readonly CliBulkSkip[];
  createdAt: string;
  expiresAt: string;
}

/**
 * What the user chose. Passed through to the backend unchanged.
 *
 * `action: null` means "move this tool to the chosen version", and the backend derives whether that
 * is an install, an upgrade, or a downgrade. A caller that decided the direction itself would need
 * its own version comparison, and two comparisons disagree the first time a prerelease appears.
 * Name an action only for `uninstall` and `repair`, which carry no version.
 */
export interface PrepareCliActionInput {
  agentId: string;
  action: string | null;
  sourceId: string;
  targetVersion: string | null;
  channel: string | null;
}

/** What execution submits. Deliberately not enough to rebuild a command from. */
export interface ExecuteCliActionInput {
  planId: string;
  expectedRevision: number;
}
