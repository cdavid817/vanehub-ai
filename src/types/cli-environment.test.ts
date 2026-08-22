import { describe, expect, it } from "vitest";
import {
  CLI_MUTATION_OUTCOMES,
  CLI_OPERATION_PHASES,
  CLI_OUTCOMES_THAT_MAY_HAVE_CHANGED_THE_MACHINE,
  CLI_PLAN_REJECTION_CODES,
  type CliMutationOutcome,
  type CliMutationResult,
} from "./cli-environment";

/**
 * Cross-language drift guard.
 *
 * Each list below is the exact set of wire strings the Rust side emits, asserted there by
 * `CliMutationOutcome::as_str`, `CliPlanRejection::as_str`, and the CLI phase constants. Renaming
 * a variant on either side fails one of the two suites, so the pair cannot drift silently.
 */
describe("CLI environment contract", () => {
  it("carries exactly the five mutation outcomes the Rust domain emits", () => {
    expect([...CLI_MUTATION_OUTCOMES]).toEqual([
      "verified",
      "applied-unverified",
      "changed-but-failed",
      "no-change-failed",
      "cancelled",
    ]);
  });

  it("separates outcomes that changed the machine from ones that did not", () => {
    // `applied-unverified` is in this list precisely because the command ran. Treating it as a
    // no-op is what let a stale snapshot be written back over a machine that had already changed.
    expect([...CLI_OUTCOMES_THAT_MAY_HAVE_CHANGED_THE_MACHINE]).toEqual([
      "verified",
      "applied-unverified",
      "changed-but-failed",
    ]);

    const unchanged = CLI_MUTATION_OUTCOMES.filter(
      (outcome) => !CLI_OUTCOMES_THAT_MAY_HAVE_CHANGED_THE_MACHINE.includes(outcome),
    );
    expect(unchanged).toEqual(["no-change-failed", "cancelled"]);
  });

  it("carries the plan rejection codes the backend returns", () => {
    expect([...CLI_PLAN_REJECTION_CODES]).toEqual([
      "plan-revision-mismatch",
      "plan-expired",
      "plan-consumed",
      "plan-stale",
    ]);
  });

  it("carries the CLI operation phases", () => {
    expect([...CLI_OPERATION_PHASES]).toEqual([
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
    ]);
    // Phase is descriptive; it must not overlap with the authoritative lifecycle statuses.
    for (const status of ["queued", "running", "succeeded", "failed"]) {
      expect(CLI_OPERATION_PHASES).not.toContain(status);
    }
  });

  it("types a terminal result that names its source and target", () => {
    const result: CliMutationResult = {
      agentId: "claude-code",
      sourceId: "npm",
      action: "upgrade",
      targetVersion: "1.3.0",
      outcome: "applied-unverified",
      warningCode: "verification-failed",
    };

    expect(result.outcome satisfies CliMutationOutcome).toBe("applied-unverified");
    expect(CLI_MUTATION_OUTCOMES).toContain(result.outcome);
    // A result records the source that ran, so a UI can state it without inferring one.
    expect(result.sourceId).toBe("npm");
  });

  it("allows a result with no target version for actions that carry none", () => {
    const uninstall: CliMutationResult = {
      agentId: "codex-cli",
      sourceId: "npm",
      action: "uninstall",
      targetVersion: null,
      outcome: "verified",
      warningCode: null,
    };
    expect(uninstall.targetVersion).toBeNull();
    expect(uninstall.warningCode).toBeNull();
  });
});
