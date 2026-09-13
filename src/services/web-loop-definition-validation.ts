import { i18n } from "../i18n";
import type { SaveLoopDefinitionInput } from "../types/loop";
import { mockAgents } from "./mock-agent-data";

/**
 * Web-side mirror of the native definition validator. It rejects the same shapes the Rust
 * domain rejects so the mock never persists a definition the desktop would refuse.
 */
export function validateLoopDefinitionInput(input: SaveLoopDefinitionInput) {
  const name = input.name.trim();
  const projectPath = input.projectPath.trim();
  const baseBranch = input.baseBranch.trim();
  const goal = input.goal.trim();
  if (!name || !projectPath || !baseBranch || !goal) throw new Error(i18n.t("loops.editor.error.scope"));
  if (!mockAgents.some((agent) => agent.id === input.workerAgentId)) throw new Error(i18n.t("loops.web.error.unsupportedWorker", { agentId: input.workerAgentId }));
  if (!mockAgents.some((agent) => agent.id === input.verifierAgentId)) throw new Error(i18n.t("loops.web.error.unsupportedVerifier", { agentId: input.verifierAgentId }));
  if (input.acceptanceCriteria.every((criterion) => !criterion.trim())) throw new Error(i18n.t("loops.editor.error.acceptance"));
  if (input.verificationCommands.length === 0) throw new Error(i18n.t("loops.editor.error.verificationRequired"));
  for (const command of input.verificationCommands) {
    if (!command.id.trim() || !command.program.trim() || command.timeoutSeconds < 1) throw new Error(i18n.t("loops.web.error.invalidCommand"));
    const workingDirectory = command.workingDirectory?.trim() ?? null;
    if (workingDirectory && (/^(?:[a-zA-Z]:[\\/]|[\\/])/.test(workingDirectory) || workingDirectory.split(/[\\/]+/).includes(".."))) {
      throw new Error(i18n.t("loops.editor.error.verificationDirectory"));
    }
    if (command.kind === "native-check" && (command.program.trim() !== "patch-whitespace" || command.args.length > 0)) {
      throw new Error(i18n.t("loops.editor.error.nativeCheck"));
    }
  }
  const scopeSchemaVersion = input.scopeSchemaVersion ?? null;
  const allowedPaths = input.allowedPaths.map((value) => value.trim()).filter(Boolean);
  const protectedPaths = input.protectedPaths.map((value) => value.trim()).filter(Boolean);
  if (scopeSchemaVersion !== null) {
    if (allowedPaths.length === 0) throw new Error(i18n.t("loops.editor.error.allowedPaths"));
    for (const entry of [...allowedPaths, ...protectedPaths]) {
      if (entry.split(/[\\/]+/).includes("..") || /^(?:[a-zA-Z]:|[\\/])/.test(entry) || /[*?[\]{}]/.test(entry)) {
        throw new Error(i18n.t("loops.editor.error.scopeSyntax", { entry }));
      }
    }
    if (allowedPaths.some((entry) => entry.split(/[\\/]+/).some((component) => component.toLowerCase() === ".git"))) {
      throw new Error(i18n.t("loops.editor.error.scopeReserved"));
    }
    if (allowedPaths.some((entry) => protectedPaths.some((shield) => entry === shield || entry.startsWith(`${shield}/`)))) {
      throw new Error(i18n.t("loops.editor.error.scopeCovered"));
    }
  }
  const { limits } = input;
  if (
    limits.maxIterations < 1 || limits.maxIterations > 20 ||
    limits.stepTimeoutSeconds < 1 || limits.totalTimeoutSeconds < limits.stepTimeoutSeconds ||
    limits.maxConsecutiveRuntimeErrors < 1 || limits.maxConsecutiveNoProgress < 1
  ) throw new Error(i18n.t("loops.editor.error.limits"));
  return {
    ...input,
    name,
    projectPath,
    baseBranch,
    goal,
    acceptanceCriteria: input.acceptanceCriteria.map((value) => value.trim()).filter(Boolean),
    allowedPaths,
    protectedPaths,
    verificationCommands: input.verificationCommands.map((command) => ({
      ...command,
      kind: command.kind ?? "process",
      id: command.id.trim(),
      program: command.program.trim(),
      args: command.args.map((value) => value.trim()).filter(Boolean),
      workingDirectory: command.workingDirectory?.trim() || null,
    })),
    limits: { ...input.limits },
    scopeSchemaVersion,
    requestedMode: scopeSchemaVersion === null ? null : input.requestedMode ?? "preventive-required",
    scopeState: scopeSchemaVersion === null ? "legacy-unverified" as const : "verified" as const,
  };
}
