import { i18n } from "../i18n";
import type { Session } from "../types/agent";
import type { LoopIteration, LoopRun } from "../types/loop";
import { nowIso } from "./web-mock-clock";
import { prependWebSession } from "./web-session-state";
import { projectWebOwnerRun } from "./web-agent-run-state";
import {
  addLoopEvidence,
  addWebLoopRoleSession,
  clearWebLoopTimer,
  createWebLoopIteration,
  currentLoopIteration,
  emitLoopEvent,
  getWebLoopPhaseDelayMs,
  getWebLoopTimer,
  isWebLoopRoleSession,
  setWebLoopTimer,
} from "./web-loop-state";

export function createWebLoopRoleSession(run: LoopRun, iteration: LoopIteration, role: "worker" | "verifier") {
  const sessionId = role === "worker" ? iteration.workerSessionId : iteration.verifierSessionId;
  if (!sessionId || isWebLoopRoleSession(sessionId)) return;
  const timestamp = nowIso();
  const agentId = role === "worker"
    ? run.definitionSnapshot.workerAgentId
    : run.definitionSnapshot.verifierAgentId;
  const session: Session = {
    id: sessionId,
    title: `${run.definitionSnapshot.name} - ${i18n.t(`loops.inspection.role.${role}`)}`,
    agentId,
    interactionMode: "cli",
    personalizationMode: "standard", lifecycleState: "stopped",
    recoveryStatus: "clean",
    recoveryRevision: 0,
    stateRevision: 0,
    historyRevision: 0,
    activeExecutionRunId: null,
    folder: run.worktreePath,
    projectPath: run.projectPath,
    worktreePath: run.worktreePath,
    worktreeName: run.worktreeName,
    worktreeBranch: run.worktreeBranch,
    remoteWorkspace: null,
    remoteSshConnectionId: null,
    remoteSshConnectionRevision: null,
    runtimeSessionId: null,
    categoryId: null,
    source: { kind: "desktop", connector: null },
    pinned: false,
    archived: false,
    createdAt: timestamp,
    updatedAt: timestamp,
  };
  addWebLoopRoleSession(sessionId);
  prependWebSession(session);
}

export function scheduleWebLoopPhase(run: LoopRun) {
  const existing = getWebLoopTimer(run.id);
  if (existing) clearTimeout(existing);
  const timeoutId = setTimeout(() => {
    clearWebLoopTimer(run.id);
    if (run.status !== "queued" && run.status !== "running") return;
    if (run.pauseRequested) {
      run.pauseRequested = false;
      run.status = "paused";
      projectWebOwnerRun(run.id, "paused");
      emitLoopEvent(run);
      return;
    }

    if (run.status === "queued") {
      run.status = "running";
      run.startedAt = nowIso();
      run.worktreeName = `loop-${run.definitionId}-${run.id}`;
      run.worktreeBranch = `vanehub/${run.worktreeName}`;
      run.worktreePath = `${run.projectPath}-${run.worktreeName}`;
      run.phase = "acting";
      projectWebOwnerRun(run.id, "running");
      const iteration = createWebLoopIteration(run.id, 1, null);
      run.iterations.push(iteration);
      createWebLoopRoleSession(run, iteration, "worker");
      addLoopEvidence(run, null, {
        kind: "worktree",
        status: "passed",
        summary: i18n.t("loops.web.evidence.worktreePrepared"),
        operationId: run.activeOperationId,
        commandId: null,
        exitCode: 0,
        durationMs: 180,
        details: { simulated: true, path: run.worktreePath },
      });
      scheduleWebLoopPhase(run);
      return;
    }

    const iteration = currentLoopIteration(run);
    if (run.phase === "acting") {
      iteration.workerSummary = i18n.t("loops.web.evidence.workerCompleted");
      iteration.diffFingerprint = `mock-diff-${run.id}-${iteration.sequence}`;
      addLoopEvidence(run, iteration, {
        kind: "worker",
        status: "passed",
        summary: iteration.workerSummary,
        operationId: `web-loop-worker-operation-${run.id}-${iteration.sequence}`,
        commandId: null,
        exitCode: 0,
        durationMs: 420,
        details: { simulated: true, changedFiles: 3, additions: 48, deletions: 12 },
      });
      run.phase = "verifying";
      projectWebOwnerRun(run.id, "verifying");
      emitLoopEvent(run, "iteration-updated");
      scheduleWebLoopPhase(run);
      return;
    }

    if (run.phase === "verifying") {
      run.definitionSnapshot.verificationCommands.forEach((command) => {
        const failed = command.program.toLowerCase() === "false";
        addLoopEvidence(run, iteration, {
          kind: "verification",
          status: failed ? "failed" : "passed",
          summary: `${command.program} ${command.args.join(" ")}`.trim(),
          operationId: `web-loop-check-${run.id}-${iteration.sequence}-${command.id}`,
          commandId: command.id,
          exitCode: failed ? 1 : 0,
          durationMs: 240,
          details: { simulated: true, required: command.required },
        });
      });
      const requiredCheckFailed = iteration.evidence.some(
        (evidence) => evidence.kind === "verification" && evidence.status === "failed" && evidence.details?.required === true,
      );
      iteration.verifierSessionId = `web-loop-verifier-${run.id}-${iteration.sequence}`;
      createWebLoopRoleSession(run, iteration, "verifier");
      iteration.verifierRecommendation = requiredCheckFailed ? "revise" : "pass";
      iteration.verifierFindings = requiredCheckFailed
        ? [i18n.t("loops.web.evidence.requiredCheckFailed")]
        : [i18n.t("loops.web.evidence.checksPassed"), i18n.t("loops.web.evidence.protectedPathsUnchanged")];
      addLoopEvidence(run, iteration, {
        kind: "verifier",
        status: requiredCheckFailed ? "blocked" : "passed",
        summary: requiredCheckFailed
          ? i18n.t("loops.web.evidence.verifierRevise")
          : i18n.t("loops.web.evidence.verifierAccept"),
        operationId: `web-loop-verifier-operation-${run.id}-${iteration.sequence}`,
        commandId: null,
        exitCode: null,
        durationMs: 320,
        details: { simulated: true, recommendation: iteration.verifierRecommendation },
      });
      run.phase = "deciding";
      emitLoopEvent(run, "iteration-updated");
      scheduleWebLoopPhase(run);
      return;
    }

    if (run.phase === "deciding") {
      const requiredCheckFailed = iteration.evidence.some(
        (evidence) => evidence.kind === "verification" && evidence.status === "failed" && evidence.details?.required === true,
      );
      iteration.status = requiredCheckFailed ? "failed" : "awaiting-acceptance";
      iteration.decisionReason = requiredCheckFailed
        ? i18n.t("loops.web.evidence.decisionCheckFailed")
        : i18n.t("loops.web.evidence.decisionReady");
      iteration.completedAt = nowIso();
      run.status = requiredCheckFailed ? "failed" : "awaiting-acceptance";
      projectWebOwnerRun(run.id, requiredCheckFailed ? "failed" : "verifying");
      run.phase = "finalizing";
      run.terminalReason = requiredCheckFailed ? "verification-failed" : null;
      run.completedAt = requiredCheckFailed ? nowIso() : null;
      addLoopEvidence(run, iteration, {
        kind: "decision",
        status: requiredCheckFailed ? "failed" : "passed",
        summary: iteration.decisionReason,
        operationId: null,
        commandId: null,
        exitCode: null,
        durationMs: null,
        details: { simulated: true, decision: run.status },
      });
    }
  }, getWebLoopPhaseDelayMs());
  setWebLoopTimer(run.id, timeoutId);
}
