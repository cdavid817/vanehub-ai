import { describe, expect, it } from "vitest";
import {
  CLI_MUTATION_OUTCOMES,
  CLI_OPERATION_PHASES,
  CLI_OPERATION_TERMINATIONS,
  CLI_OUTCOMES_THAT_MAY_HAVE_CHANGED_THE_MACHINE,
  CLI_PLAN_REJECTION_CODES,
  CLI_VERIFICATION_WARNINGS,
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

  it("carries exactly the terminations `CliOperationTermination::as_str` emits", () => {
    expect([...CLI_OPERATION_TERMINATIONS]).toEqual([
      "not-started",
      "exited",
      "exited-without-code",
      "timed-out",
      "cancelled",
    ]);
  });

  it("carries exactly the verification warnings `CliVerificationWarning::as_str` emits", () => {
    expect([...CLI_VERIFICATION_WARNINGS]).toEqual([
      "detection-skipped-while-busy",
      "detection-failed",
      "target-version-not-observed",
    ]);
  });

  it("keeps termination and outcome as separate vocabularies", () => {
    // `cancelled` appears in both, and means different things: the process was interrupted, versus
    // nothing on the machine changed. A UI that switched on one field for both would conflate an
    // interrupted-but-applied change with a clean no-op.
    const shared = CLI_OPERATION_TERMINATIONS.filter((termination) =>
      (CLI_MUTATION_OUTCOMES as readonly string[]).includes(termination),
    );
    expect(shared).toEqual(["cancelled"]);
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
      operationId: "op-1",
      agentId: "claude-code",
      sourceId: "npm",
      action: "upgrade",
      targetVersion: "1.3.0",
      observedVersion: "1.2.0",
      phase: "completed",
      termination: "exited",
      exitCode: 0,
      elapsedMs: 1200,
      outcome: "applied-unverified",
      warnings: ["target-version-not-observed"],
      outputTruncated: false,
      warning: true,
    };

    expect(result.outcome satisfies CliMutationOutcome).toBe("applied-unverified");
    expect(CLI_MUTATION_OUTCOMES).toContain(result.outcome);
    // A result records the source that ran, so a UI can state it without inferring one.
    expect(result.sourceId).toBe("npm");
    // The command exited 0 and verification did not confirm it. Both are true at once, which is
    // why termination and outcome are separate fields.
    expect(result.exitCode).toBe(0);
    expect(result.observedVersion).not.toBe(result.targetVersion);
  });

  it("allows a result with no target version for actions that carry none", () => {
    const uninstall: CliMutationResult = {
      operationId: "op-2",
      agentId: "codex-cli",
      sourceId: "npm",
      action: "uninstall",
      targetVersion: null,
      observedVersion: null,
      phase: "completed",
      termination: "exited",
      exitCode: 0,
      elapsedMs: 800,
      outcome: "verified",
      warnings: [],
      outputTruncated: false,
      warning: false,
    };
    expect(uninstall.targetVersion).toBeNull();
    expect(uninstall.warnings).toHaveLength(0);
    expect(uninstall.warning).toBe(false);
  });

  it("reports no exit code for a cancelled or timed-out process", () => {
    // Inventing a code for a process that never reported one would put a number on screen that no
    // process ever produced.
    const cancelled: CliMutationResult = {
      operationId: "op-3",
      agentId: "claude-code",
      sourceId: "npm",
      action: "upgrade",
      targetVersion: "1.3.0",
      observedVersion: "1.2.0",
      phase: "completed",
      termination: "cancelled",
      exitCode: null,
      elapsedMs: 400,
      outcome: "cancelled",
      warnings: [],
      outputTruncated: false,
      warning: true,
    };

    expect(cancelled.exitCode).toBeNull();
    expect(CLI_OPERATION_TERMINATIONS).toContain(cancelled.termination);
    expect(CLI_VERIFICATION_WARNINGS).not.toContain(cancelled.termination as never);
  });
});
