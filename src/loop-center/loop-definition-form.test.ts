import { describe, expect, it } from "vitest";
import type { LoopDefinition } from "../types/loop";
import { createLoopDefinitionDraft, createVerificationCommandDraft, toSaveLoopDefinitionInput, validateLoopDefinitionStep, validateVerificationCommand } from "./loop-definition-form";

describe("Loop definition form", () => {
  it("creates a runnable draft with bounded default limits", () => {
    const draft = createLoopDefinitionDraft();

    expect(draft.baseBranch).toBe("main");
    expect(draft.verificationCommands[0]).toMatchObject({ program: "npm", arguments: "run\ntest", required: true });
    expect(draft.limits).toMatchObject({ maxIterations: 3, stepTimeoutSeconds: 300, totalTimeoutSeconds: 1800 });
  });

  it("validates each editable step independently", () => {
    const draft = createLoopDefinitionDraft();
    expect(validateLoopDefinitionStep(draft, 0)).toBe("scope");

    Object.assign(draft, { name: "Release", projectPath: "D:/repo", goal: "Ship", acceptanceCriteria: "Tests pass" });
    expect(validateLoopDefinitionStep(draft, 0)).toBeNull();
    expect(validateLoopDefinitionStep(draft, 1)).toBe("agents");

    Object.assign(draft, { workerAgentId: "codex-cli", verifierAgentId: "claude-code" });
    expect(validateLoopDefinitionStep(draft, 1)).toBeNull();
    draft.limits.totalTimeoutSeconds = 10;
    expect(validateLoopDefinitionStep(draft, 2)).toBe("limits");
  });

  it("reports every step-specific validation boundary", () => {
    const draft = createLoopDefinitionDraft();
    Object.assign(draft, {
      name: "Release",
      projectPath: "D:/repo",
      baseBranch: "main",
      goal: "Ship",
      acceptanceCriteria: "Tests pass",
      workerAgentId: "codex-cli",
      verifierAgentId: "claude-code",
    });

    draft.acceptanceCriteria = " \n ";
    expect(validateLoopDefinitionStep(draft, 0)).toBe("acceptance");
    draft.acceptanceCriteria = "Tests pass";
    expect(validateLoopDefinitionStep(draft, 0)).toBeNull();

    draft.workerAgentId = "";
    expect(validateLoopDefinitionStep(draft, 1)).toBe("agents");
    draft.workerAgentId = "codex-cli";
    expect(validateLoopDefinitionStep(draft, 1)).toBeNull();

    draft.verificationCommands = [];
    expect(validateLoopDefinitionStep(draft, 2)).toBe("verificationRequired");
    draft.verificationCommands = [{ ...createVerificationCommandDraft([]), program: "npm" }];
    const invalidLimits = [
      { maxIterations: 0 },
      { maxIterations: 21 },
      { stepTimeoutSeconds: 0 },
      { totalTimeoutSeconds: 299 },
      { maxConsecutiveRuntimeErrors: 0 },
      { maxConsecutiveNoProgress: 0 },
    ];
    for (const invalid of invalidLimits) {
      draft.limits = { ...createLoopDefinitionDraft().limits, ...invalid };
      expect(validateLoopDefinitionStep(draft, 2)).toBe("limits");
    }
  });

  it("uses optimistic versioning and preserves all structured verification commands", () => {
    const definition = exampleDefinition();
    const draft = createLoopDefinitionDraft(definition);
    draft.verificationCommands[0].program = "npm";
    draft.verificationCommands[0].arguments = "run\nlint";

    const input = toSaveLoopDefinitionInput(draft, definition);

    expect(input.expectedVersion).toBe(4);
    expect(input.verificationCommands).toHaveLength(2);
    expect(input.verificationCommands[0]).toMatchObject({ id: "tests", program: "npm", args: ["run", "lint"] });
    expect(input.verificationCommands[1].id).toBe("lint");
  });

  it("rejects unsafe working directories and invalid command timeouts", () => {
    const command = createVerificationCommandDraft([]);
    command.program = "npm";
    command.workingDirectory = "../outside";
    expect(validateVerificationCommand(command)).toBe("verificationDirectory");

    command.workingDirectory = "packages/app";
    command.timeoutSeconds = 0;
    expect(validateVerificationCommand(command)).toBe("verificationTimeout");
    command.timeoutSeconds = 30;
    expect(validateVerificationCommand(command)).toBeNull();
  });

  it.each(["/tmp", "\\server\\share", "C:\\repo", "packages/../outside"])(
    "rejects absolute or escaping verification directory %s",
    (workingDirectory) => {
      const command = { ...createVerificationCommandDraft([]), program: "npm", workingDirectory };
      expect(validateVerificationCommand(command)).toBe("verificationDirectory");
    },
  );

  it("requires a program and a positive integer timeout", () => {
    const command = createVerificationCommandDraft([]);
    expect(validateVerificationCommand(command)).toBe("verificationProgram");
    command.program = "npm";
    for (const timeoutSeconds of [-1, 0, 1.5]) {
      command.timeoutSeconds = timeoutSeconds;
      expect(validateVerificationCommand(command)).toBe("verificationTimeout");
    }
  });

  it("trims scalar and line-based values when serializing", () => {
    const draft = createLoopDefinitionDraft();
    Object.assign(draft, {
      name: "  Release  ", projectPath: " D:/repo ", baseBranch: " main ", goal: " Ship ",
      acceptanceCriteria: " Tests pass \n\n Docs updated ", allowedPaths: " src \n tests ",
      protectedPaths: " .git \n ", workerAgentId: "codex-cli", verifierAgentId: "claude-code",
    });
    draft.verificationCommands = [{
      ...createVerificationCommandDraft([]),
      program: " npm ",
      arguments: " run \n\n test ",
      workingDirectory: " packages/app ",
    }];

    expect(toSaveLoopDefinitionInput(draft)).toMatchObject({
      name: "Release",
      projectPath: "D:/repo",
      baseBranch: "main",
      goal: "Ship",
      acceptanceCriteria: ["Tests pass", "Docs updated"],
      allowedPaths: ["src", "tests"],
      protectedPaths: [".git"],
      expectedVersion: null,
      verificationCommands: [{
        program: "npm", args: ["run", "test"], workingDirectory: "packages/app",
      }],
    });
  });

  it("creates stable unique ids for appended command rows", () => {
    const first = createVerificationCommandDraft([]);
    const third = createVerificationCommandDraft([first, { ...first, id: "verification-2" }]);
    expect(third.id).toBe("verification-3");
  });
});

function exampleDefinition(): LoopDefinition {
  return {
    id: "loop-1", name: "Release", enabled: true, projectPath: "D:/repo", baseBranch: "main", goal: "Ship",
    acceptanceCriteria: ["Tests pass"], allowedPaths: ["src"], protectedPaths: [".git"], workerAgentId: "codex-cli", verifierAgentId: "claude-code",
    verificationCommands: [
      { id: "tests", program: "npm", args: ["test"], workingDirectory: null, timeoutSeconds: 120, required: true },
      { id: "lint", program: "npm", args: ["run", "lint"], workingDirectory: null, timeoutSeconds: 120, required: false },
    ],
    limits: { maxIterations: 3, stepTimeoutSeconds: 300, totalTimeoutSeconds: 1800, maxConsecutiveRuntimeErrors: 2, maxConsecutiveNoProgress: 2 },
    version: 4, createdAt: "2026-07-22T00:00:00Z", updatedAt: "2026-07-22T00:00:00Z",
  };
}
