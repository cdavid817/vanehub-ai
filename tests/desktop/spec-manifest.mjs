/**
 * Which desktop gate each spec belongs to.
 *
 * Two gates, because one suite cannot be both hermetic and a real-integration test. The required
 * gate runs on every pull request against fixture CLI Agents, a temporary HOME and a temporary
 * database; the external suite verifies the parts only a real vendor binary, login, or model
 * response can verify, and runs outside the gate.
 *
 * This file is the source of truth, and `desktop-spec-manifest.node-test.mjs` enforces it: a spec
 * that exists without an entry, an entry without a spec, a required spec declaring an external
 * prerequisite, or an external spec reaching the required command all fail the desktop unit tests.
 * A classification kept only in prose drifts the first time someone adds a file.
 */

/** Runs in the required gate, against fixtures. Any failure fails the gate. */
export const REQUIRED_FIXTURE = "required-fixture";
/** Runs only when a real Agent, credential, or provider is present. Never gates a pull request. */
export const EXTERNAL_PROVIDER = "external-provider";
/** Deleted because a dedicated layer covers the same behaviour better. */
export const DUPLICATE_REPLACED = "duplicate-replaced";

/**
 * Environment variables that mean "a real external thing is required".
 *
 * A required spec naming any of these is a classification error: whatever it gates behind them is
 * either fixture-resolvable and should be fixtured, or genuinely external and belongs in the other
 * suite.
 */
export const EXTERNAL_PREREQUISITE_VARIABLES = [
  "VANEHUB_DESKTOP_MUTATE_HOST",
  "VANEHUB_SSH_HOST",
  "VANEHUB_SSH_USER",
  "VANEHUB_SSH_PASSWORD",
];

export const DESKTOP_SPECS = [
  { spec: "domain-app-updates.e2e.mjs", gate: REQUIRED_FIXTURE },
  { spec: "domain-automation.e2e.mjs", gate: REQUIRED_FIXTURE },
  { spec: "domain-cli-tooling.e2e.mjs", gate: REQUIRED_FIXTURE },
  { spec: "domain-evaluation.e2e.mjs", gate: REQUIRED_FIXTURE },
  { spec: "domain-floating-assistant.e2e.mjs", gate: REQUIRED_FIXTURE },
  { spec: "domain-loop.e2e.mjs", gate: REQUIRED_FIXTURE },
  { spec: "domain-lsp.e2e.mjs", gate: REQUIRED_FIXTURE },
  { spec: "domain-multi-agent-business.e2e.mjs", gate: REQUIRED_FIXTURE },
  { spec: "domain-multi-agent-human-decision.e2e.mjs", gate: REQUIRED_FIXTURE },
  { spec: "domain-multi-agent-project.e2e.mjs", gate: REQUIRED_FIXTURE },
  { spec: "domain-multi-agent-routing.e2e.mjs", gate: REQUIRED_FIXTURE },
  { spec: "domain-multi-agent.e2e.mjs", gate: REQUIRED_FIXTURE },
  { spec: "domain-observability.e2e.mjs", gate: REQUIRED_FIXTURE },
  { spec: "domain-prompt-hooks.e2e.mjs", gate: REQUIRED_FIXTURE },
  { spec: "domain-read-surface.e2e.mjs", gate: REQUIRED_FIXTURE },
  { spec: "domain-skills.e2e.mjs", gate: REQUIRED_FIXTURE },
  { spec: "domain-work-board.e2e.mjs", gate: REQUIRED_FIXTURE },
  { spec: "domain-worktree.e2e.mjs", gate: REQUIRED_FIXTURE },
  { spec: "feature-sweep.e2e.mjs", gate: REQUIRED_FIXTURE },
  { spec: "screen-sweep.e2e.mjs", gate: REQUIRED_FIXTURE },
  { spec: "sessions.e2e.mjs", gate: REQUIRED_FIXTURE },
  { spec: "smoke.e2e.mjs", gate: REQUIRED_FIXTURE },
  { spec: "ui-agent-configuration.e2e.mjs", gate: REQUIRED_FIXTURE },
  { spec: "ui-chat.e2e.mjs", gate: REQUIRED_FIXTURE },
  { spec: "ui-evaluation.e2e.mjs", gate: REQUIRED_FIXTURE },
  { spec: "ui-multi-agent.e2e.mjs", gate: REQUIRED_FIXTURE },
  { spec: "ui-notifications.e2e.mjs", gate: REQUIRED_FIXTURE },
  { spec: "ui-settings.e2e.mjs", gate: REQUIRED_FIXTURE },
  { spec: "ui-workspace.e2e.mjs", gate: REQUIRED_FIXTURE },

  // The one spec that drives real package managers, a real host Python environment, and a real SSH
  // server. Every case in it already refuses to run without an explicit opt-in variable, which is
  // the shape of an external-suite spec, not of a gate spec.
  {
    spec: "native-flows.e2e.mjs",
    gate: EXTERNAL_PROVIDER,
    prerequisites: ["VANEHUB_DESKTOP_MUTATE_HOST", "VANEHUB_SSH_HOST", "VANEHUB_SSH_USER", "VANEHUB_SSH_PASSWORD"],
    blockedReason:
      "Reinstalls a global CLI through the real npm, mutates the host Python environment, and opens a real SSH session.",
  },

  // Both drove the developer's own machine to check CLI discovery and the management page. The
  // `desktop-cli-management` layer covers the same ground -- discovery, planning, execution,
  // verification, and the page reflecting the result -- against a fixture PATH, which is strictly
  // better evidence, so these were deleted rather than reclassified.
  {
    spec: "ui-cli-management.e2e.mjs",
    gate: DUPLICATE_REPLACED,
    replacedBy: "specs-cli-management/cli-lifecycle.e2e.mjs",
  },
  {
    spec: "domain-cli-install.e2e.mjs",
    gate: DUPLICATE_REPLACED,
    replacedBy: "specs-cli-management/cli-lifecycle.e2e.mjs",
  },
];

/** Spec file names the required gate runs, in manifest order. */
export function requiredSpecFiles() {
  return DESKTOP_SPECS.filter((entry) => entry.gate === REQUIRED_FIXTURE).map((entry) => entry.spec);
}

/** Spec file names the external suite runs. */
export function externalSpecFiles() {
  return DESKTOP_SPECS.filter((entry) => entry.gate === EXTERNAL_PROVIDER).map((entry) => entry.spec);
}

/** Entries whose spec file should no longer exist. */
export function replacedSpecs() {
  return DESKTOP_SPECS.filter((entry) => entry.gate === DUPLICATE_REPLACED);
}
