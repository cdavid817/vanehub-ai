import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import "../i18n";
import type { SaveLoopDefinitionInput } from "../types/loop";
import type { LoopWorkbenchService } from "./loop-service";
import { resetWebLoopsForTest, webAgentClient as baseWebAgentClient } from "./web-agent-client";
import { webLoopClient } from "./web-loop-client";

const webAgentClient: LoopWorkbenchService = { ...baseWebAgentClient, ...webLoopClient };

const baseInput: SaveLoopDefinitionInput = {
  name: "Scope Loop", enabled: true, projectPath: "D:/example-workspace", baseBranch: "main", goal: "Exercise scope gates",
  acceptanceCriteria: ["Checks pass"], allowedPaths: ["src"], protectedPaths: [".git"], workerAgentId: "onepiece", verifierAgentId: "onepiece",
  verificationCommands: [{ id: "whitespace", kind: "native-check", program: "patch-whitespace", args: [], workingDirectory: null, timeoutSeconds: 60, required: true }],
  limits: { maxIterations: 3, stepTimeoutSeconds: 60, totalTimeoutSeconds: 600, maxConsecutiveRuntimeErrors: 2, maxConsecutiveNoProgress: 2 },
  scopeSchemaVersion: 1,
};

describe("Web Loop scope simulation", () => {
  beforeEach(() => { resetWebLoopsForTest(); vi.useFakeTimers(); });
  afterEach(() => vi.useRealTimers());

  it("defaults saved definitions to strict enforcement and marks unversioned ones legacy", async () => {
    const strict = await webAgentClient.createLoopDefinition(baseInput);
    expect(strict).toMatchObject({ scopeSchemaVersion: 1, requestedMode: "preventive-required", scopeState: "verified" });
    const legacy = await webAgentClient.createLoopDefinition({ ...baseInput, scopeSchemaVersion: null, requestedMode: "artifact-audited" });
    expect(legacy).toMatchObject({ scopeSchemaVersion: null, requestedMode: null, scopeState: "legacy-unverified" });
    const readiness = await webAgentClient.checkLoopReadiness(legacy.id);
    expect(readiness.ready).toBe(false);
    expect(readiness.checks.find((check) => check.code === "scope-version-supported")?.status).toBe("blocked");
    await expect(webAgentClient.startLoop(legacy.id)).rejects.toThrow();
  });

  it("rejects scope entries the native validator would refuse", async () => {
    await expect(webAgentClient.createLoopDefinition({ ...baseInput, allowedPaths: [] })).rejects.toThrow();
    await expect(webAgentClient.createLoopDefinition({ ...baseInput, allowedPaths: [".git"] })).rejects.toThrow();
    await expect(webAgentClient.createLoopDefinition({ ...baseInput, allowedPaths: ["../outside"] })).rejects.toThrow();
    await expect(webAgentClient.createLoopDefinition({ ...baseInput, allowedPaths: ["src/**"] })).rejects.toThrow();
    await expect(webAgentClient.createLoopDefinition({ ...baseInput, allowedPaths: ["src"], protectedPaths: ["src"] })).rejects.toThrow();
    await expect(webAgentClient.createLoopDefinition({ ...baseInput, verificationCommands: [{ id: "ws", kind: "native-check", program: "eslint", args: [], workingDirectory: null, timeoutSeconds: 10, required: true }] })).rejects.toThrow();
  });

  it("blocks strict mode for process checks and CLI roles alike, without an automatic downgrade", async () => {
    const withProcess = await webAgentClient.createLoopDefinition({ ...baseInput, verificationCommands: [{ id: "tests", kind: "process", program: "npm", args: ["test"], workingDirectory: null, timeoutSeconds: 60, required: true }] });
    const processReadiness = await webAgentClient.checkLoopReadiness(withProcess.id);
    expect(processReadiness.ready).toBe(false);
    expect(processReadiness.assessment?.surfaces.find((surface) => surface.surface === "verification:tests")).toMatchObject({ coverage: "artifact-validation-only", blocking: true });
    const definition = await webAgentClient.createLoopDefinition({ ...baseInput, workerAgentId: "codex-cli" });
    const readiness = await webAgentClient.checkLoopReadiness(definition.id);
    expect(readiness.ready).toBe(false);
    expect(readiness.assessment?.satisfiesRequestedMode).toBe(false);
    expect(readiness.assessment?.surfaces.find((surface) => surface.surface === "worker")).toMatchObject({ coverage: "artifact-validation-only", blocking: true });
    await expect(webAgentClient.startLoop(definition.id, { expectedRevision: definition.version })).rejects.toThrow();
    expect(await webAgentClient.listLoopRuns(definition.id)).toEqual([]);
  });

  it("never admits a CLI Verifier in either mode", async () => {
    const definition = await webAgentClient.createLoopDefinition({ ...baseInput, requestedMode: "artifact-audited", verifierAgentId: "claude-code" });
    const readiness = await webAgentClient.checkLoopReadiness(definition.id);
    expect(readiness.assessment?.surfaces.find((surface) => surface.surface === "verifier")?.coverage).toBe("unsupported");
    expect(readiness.ready).toBe(false);
    const admission = await webAgentClient.prepareLoopAdmission({ action: "start", definitionId: definition.id, expectedRevision: definition.version });
    expect(admission.assessment.satisfiesRequestedMode).toBe(false);
    expect(admission.challengeId).toBeNull();
  });

  it("requires a one-use receipt bound to the transition in artifact-audited mode", async () => {
    const definition = await webAgentClient.createLoopDefinition({ ...baseInput, requestedMode: "artifact-audited", workerAgentId: "codex-cli" });
    const readiness = await webAgentClient.checkLoopReadiness(definition.id);
    expect(readiness.ready).toBe(true);
    expect(readiness.acknowledgementRequired).toBe(true);
    expect(readiness.assessment?.limitations.length).toBeGreaterThan(0);

    await expect(webAgentClient.startLoop(definition.id, { expectedRevision: definition.version })).rejects.toThrow();
    const admission = await webAgentClient.prepareLoopAdmission({ action: "start", definitionId: definition.id, expectedRevision: definition.version });
    expect(admission).toMatchObject({ action: "start", targetId: definition.id, acknowledgementRequired: true, simulated: true });
    expect(admission.challengeId).toBeTruthy();
    await expect(webAgentClient.prepareLoopAdmission({ action: "start", definitionId: definition.id, expectedRevision: 99 })).rejects.toThrow();

    const receipt = await webAgentClient.acknowledgeLoopAudit(admission.challengeId ?? "");
    await expect(webAgentClient.acknowledgeLoopAudit(admission.challengeId ?? "")).rejects.toThrow();
    await expect(webAgentClient.startLoop(definition.id, { expectedRevision: definition.version, auditAcknowledgementId: "forged" })).rejects.toThrow();
    const started = await webAgentClient.startLoop(definition.id, { expectedRevision: definition.version, auditAcknowledgementId: receipt.acknowledgementId });
    expect(started.run.scope).toMatchObject({ requestedMode: "artifact-audited", bindingStatus: "bound" });
    expect(started.run.scope?.assessment?.simulated).toBe(true);

    await webAgentClient.cancelLoop(started.run.id);
    // The receipt was consumed by the start; a second definition start cannot reuse it.
    await expect(webAgentClient.startLoop(definition.id, { expectedRevision: definition.version, auditAcknowledgementId: receipt.acknowledgementId })).rejects.toThrow();
  });

  it("issues resume and continue receipts against the run revision and rejects cross-action reuse", async () => {
    const definition = await webAgentClient.createLoopDefinition({ ...baseInput, requestedMode: "artifact-audited", workerAgentId: "codex-cli" });
    const startAdmission = await webAgentClient.prepareLoopAdmission({ action: "start", definitionId: definition.id, expectedRevision: definition.version });
    const startReceipt = await webAgentClient.acknowledgeLoopAudit(startAdmission.challengeId ?? "");
    const started = await webAgentClient.startLoop(definition.id, { expectedRevision: definition.version, auditAcknowledgementId: startReceipt.acknowledgementId });
    await vi.advanceTimersByTimeAsync(900);
    const awaiting = await webAgentClient.getLoopRun(started.run.id);
    expect(awaiting.status).toBe("awaiting-acceptance");

    const resumeAdmission = await webAgentClient.prepareLoopAdmission({ action: "resume", runId: awaiting.id, expectedRevision: awaiting.revision });
    const resumeReceipt = await webAgentClient.acknowledgeLoopAudit(resumeAdmission.challengeId ?? "");
    await expect(webAgentClient.continueLoop({ runId: awaiting.id, feedback: "again", envelope: { expectedRevision: awaiting.revision, auditAcknowledgementId: resumeReceipt.acknowledgementId } })).rejects.toThrow();

    const continueAdmission = await webAgentClient.prepareLoopAdmission({ action: "continue", runId: awaiting.id, expectedRevision: awaiting.revision });
    const continueReceipt = await webAgentClient.acknowledgeLoopAudit(continueAdmission.challengeId ?? "");
    const continued = await webAgentClient.continueLoop({ runId: awaiting.id, feedback: "again", envelope: { expectedRevision: awaiting.revision, auditAcknowledgementId: continueReceipt.acknowledgementId } });
    expect(continued).toMatchObject({ status: "running", currentIteration: 2, revision: awaiting.revision + 1 });
  });

  it("commits acceptance against the sealed evidence and stale revisions are refused", async () => {
    const definition = await webAgentClient.createLoopDefinition(baseInput);
    const started = await webAgentClient.startLoop(definition.id, { expectedRevision: definition.version });
    await vi.advanceTimersByTimeAsync(900);
    const awaiting = await webAgentClient.getLoopRun(started.run.id);
    await expect(webAgentClient.requestLoopAcceptance({ runId: awaiting.id, expectedRevision: awaiting.revision + 5 })).rejects.toThrow();
    await expect(webAgentClient.requestLoopAcceptance({ runId: awaiting.id, expectedScopeDigest: "sha256:other" })).rejects.toThrow();
    const accepted = await webAgentClient.requestLoopAcceptance({ runId: awaiting.id, expectedRevision: awaiting.revision, expectedScopeDigest: awaiting.scope?.scopeDigest });
    expect(accepted.run).toMatchObject({ status: "succeeded", terminalReason: "goal-met" });
    expect(accepted.run.scope?.sealedEvidenceId).toBeTruthy();
    expect(accepted.run.iterations[0].evidence.map((item) => item.kind)).toEqual(["worker", "verification", "verifier", "decision", "scope-evidence", "acceptance"]);
    expect(accepted.run.scope?.assessment?.surfaces.map((surface) => surface.coverage)).not.toContain("artifact-validation-only");
    await expect(webAgentClient.requestLoopAcceptance({ runId: awaiting.id })).rejects.toThrow();
  });

  it("refuses to resume a run whose binding is missing", async () => {
    const definition = await webAgentClient.createLoopDefinition(baseInput);
    const started = await webAgentClient.startLoop(definition.id);
    await webAgentClient.pauseLoop(started.run.id);
    await vi.advanceTimersByTimeAsync(300);
    const paused = await webAgentClient.getLoopRun(started.run.id);
    expect(paused.status).toBe("paused");
    await expect(webAgentClient.resumeLoop(paused.id, { expectedRevision: paused.revision + 1 })).rejects.toThrow();
    expect((await webAgentClient.resumeLoop(paused.id, { expectedRevision: paused.revision })).status).not.toBe("paused");
  });
});
