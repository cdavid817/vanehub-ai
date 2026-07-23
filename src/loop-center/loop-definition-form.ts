import type { LoopDefinition, LoopLimits, LoopVerificationCommand, SaveLoopDefinitionInput } from "../types/loop";

export interface LoopDefinitionDraft {
  name: string;
  projectPath: string;
  baseBranch: string;
  goal: string;
  acceptanceCriteria: string;
  allowedPaths: string;
  protectedPaths: string;
  workerAgentId: string;
  verifierAgentId: string;
  verificationCommands: LoopVerificationCommandDraft[];
  limits: LoopLimits;
}

export interface LoopVerificationCommandDraft {
  id: string;
  program: string;
  arguments: string;
  workingDirectory: string;
  timeoutSeconds: number;
  required: boolean;
}

const defaultLimits: LoopLimits = {
  maxIterations: 3,
  stepTimeoutSeconds: 300,
  totalTimeoutSeconds: 1800,
  maxConsecutiveRuntimeErrors: 2,
  maxConsecutiveNoProgress: 2,
};

export function createLoopDefinitionDraft(definition?: LoopDefinition | null): LoopDefinitionDraft {
  return {
    name: definition?.name ?? "",
    projectPath: definition?.projectPath ?? "",
    baseBranch: definition?.baseBranch ?? "main",
    goal: definition?.goal ?? "",
    acceptanceCriteria: joinLines(definition?.acceptanceCriteria ?? []),
    allowedPaths: joinLines(definition?.allowedPaths ?? []),
    protectedPaths: joinLines(definition?.protectedPaths ?? []),
    workerAgentId: definition?.workerAgentId ?? "",
    verifierAgentId: definition?.verifierAgentId ?? "",
    verificationCommands: definition?.verificationCommands.map(toCommandDraft) ?? [{ ...createVerificationCommandDraft([]), program: "npm", arguments: "run\ntest" }],
    limits: definition ? { ...definition.limits } : { ...defaultLimits },
  };
}

export function validateLoopDefinitionStep(draft: LoopDefinitionDraft, step: number): string | null {
  if (step === 0 && (!draft.name.trim() || !draft.projectPath.trim() || !draft.baseBranch.trim() || !draft.goal.trim())) return "scope";
  if (step === 0 && splitLines(draft.acceptanceCriteria).length === 0) return "acceptance";
  if (step === 1 && (!draft.workerAgentId || !draft.verifierAgentId)) return "agents";
  if (step === 2 && draft.verificationCommands.length === 0) return "verificationRequired";
  if (step === 2) {
    const issue = draft.verificationCommands.map(validateVerificationCommand).find(Boolean);
    if (issue) return issue;
  }
  const limits = draft.limits;
  if (step === 2 && (limits.maxIterations < 1 || limits.maxIterations > 20 || limits.stepTimeoutSeconds < 1 || limits.totalTimeoutSeconds < limits.stepTimeoutSeconds || limits.maxConsecutiveRuntimeErrors < 1 || limits.maxConsecutiveNoProgress < 1)) return "limits";
  return null;
}

export function toSaveLoopDefinitionInput(draft: LoopDefinitionDraft, definition?: LoopDefinition | null): SaveLoopDefinitionInput {
  return {
    name: draft.name.trim(),
    enabled: definition?.enabled ?? true,
    projectPath: draft.projectPath.trim(),
    baseBranch: draft.baseBranch.trim(),
    goal: draft.goal.trim(),
    acceptanceCriteria: splitLines(draft.acceptanceCriteria),
    allowedPaths: splitLines(draft.allowedPaths),
    protectedPaths: splitLines(draft.protectedPaths),
    workerAgentId: draft.workerAgentId,
    verifierAgentId: draft.verifierAgentId,
    verificationCommands: draft.verificationCommands.map(toVerificationCommand),
    limits: { ...draft.limits },
    expectedVersion: definition?.version ?? null,
  };
}

export function createVerificationCommandDraft(commands: LoopVerificationCommandDraft[]): LoopVerificationCommandDraft {
  let sequence = commands.length + 1;
  while (commands.some((command) => command.id === `verification-${sequence}`)) sequence += 1;
  return { id: `verification-${sequence}`, program: "", arguments: "", workingDirectory: "", timeoutSeconds: 300, required: true };
}

export function validateVerificationCommand(command: LoopVerificationCommandDraft): string | null {
  if (!command.program.trim()) return "verificationProgram";
  if (!Number.isInteger(command.timeoutSeconds) || command.timeoutSeconds < 1) return "verificationTimeout";
  const directory = command.workingDirectory.trim();
  if (directory && (/^(?:[a-zA-Z]:|[\\/])/.test(directory) || directory.split(/[\\/]+/).includes(".."))) return "verificationDirectory";
  return null;
}

function splitLines(value: string) {
  return value.split(/\r?\n/).map((item) => item.trim()).filter(Boolean);
}

function joinLines(values: string[]) {
  return values.join("\n");
}

function toCommandDraft(command: LoopVerificationCommand): LoopVerificationCommandDraft {
  return {
    id: command.id,
    program: command.program,
    arguments: joinLines(command.args),
    workingDirectory: command.workingDirectory ?? "",
    timeoutSeconds: command.timeoutSeconds,
    required: command.required,
  };
}

function toVerificationCommand(command: LoopVerificationCommandDraft): LoopVerificationCommand {
  return {
    id: command.id,
    program: command.program.trim(),
    args: splitLines(command.arguments),
    workingDirectory: command.workingDirectory.trim() || null,
    timeoutSeconds: command.timeoutSeconds,
    required: command.required,
  };
}
