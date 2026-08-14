import type { BuiltinToolService } from "./builtin-tool-service";
import type {
  ArtifactDetail,
  BrowserHandoffState,
  BuiltinToolCapability,
  BuiltinToolOperation,
  BuiltinToolOperationEvent,
  DelegationAttemptSummary,
} from "../types/builtin-tools";

const simulatedAt = "2000-01-01T00:00:00.000Z";
const desktopRuntimeRequired = "desktop_runtime_required";
const operations = new Map<string, BuiltinToolOperation>();
const attempts = new Map<string, DelegationAttemptSummary>();
const handoffs = new Map<string, BrowserHandoffState>();
const listeners = new Map<string, Set<(event: BuiltinToolOperationEvent) => void>>();
let nextAttemptId = 1;

const simulatedArtifact: ArtifactDetail = {
  id: "web-simulated-artifact",
  displayName: "simulated-result.txt",
  mediaType: "text/plain",
  sizeBytes: 18,
  contentHash: "sha256:web-simulated-artifact",
  integrity: "verified",
  createdAt: simulatedAt,
  expiresAt: null,
  simulated: true,
  producerOperationId: "web-simulation",
  provenance: ["web/mock adapter", "simulated data"],
  publishedAt: null,
  publicationUrl: null,
  limitations: [desktopRuntimeRequired],
};

function emit(operation: BuiltinToolOperation) {
  listeners.get(operation.sessionId)?.forEach((listener) => {
    listener({ kind: "snapshot", operation: structuredClone(operation) });
  });
}

function requireOperation(operationId: string) {
  const operation = operations.get(operationId);
  if (!operation) throw new Error("builtin_tool_operation_not_found");
  return operation;
}

function requireArtifact(artifactId: string) {
  if (artifactId !== simulatedArtifact.id) throw new Error("artifact_not_found");
  return structuredClone(simulatedArtifact);
}

function requireAttempt(attemptId: string) {
  const attempt = attempts.get(attemptId);
  if (!attempt) throw new Error("delegation_attempt_not_found");
  return attempt;
}

function unavailableModes() {
  return (["read", "write", "execute", "publish", "apply"] as const).map((mode) => ({
    mode,
    state: "unavailable" as const,
    reasonCode: desktopRuntimeRequired,
    simulated: true,
  }));
}

const capabilities: BuiltinToolCapability[] = [
  "filesystem",
  "command",
  "browser",
  "web",
  "code_execution",
  "ocr",
  "artifact",
  "delegation",
];

export const webBuiltinToolClient: BuiltinToolService = {
  async getBuiltinToolReadiness(agentId) {
    return {
      agentId,
      observedAt: simulatedAt,
      capabilities: capabilities.map((capability) => ({ capability, modes: unavailableModes() })),
    };
  },
  async getBuiltinToolOperation(operationId) {
    return structuredClone(requireOperation(operationId));
  },
  async listBuiltinToolOperations(input) {
    return [...operations.values()]
      .filter((item) => item.sessionId === input.sessionId)
      .filter((item) => !input.capability || item.capability === input.capability)
      .slice(0, input.limit ?? 50)
      .map((item) => structuredClone(item));
  },
  async cancelBuiltinToolOperation(operationId) {
    const current = requireOperation(operationId);
    const operation = { ...current, status: "cancelled" as const, updatedAt: simulatedAt };
    operations.set(operationId, operation);
    emit(operation);
    return structuredClone(operation);
  },
  async subscribeBuiltinToolOperations(sessionId, listener) {
    const sessionListeners = listeners.get(sessionId) ?? new Set();
    sessionListeners.add(listener);
    listeners.set(sessionId, sessionListeners);
    return () => sessionListeners.delete(listener);
  },
  async listArtifacts(input) {
    const items = input.cursor ? [] : [structuredClone(simulatedArtifact)];
    return { items: items.slice(0, input.limit ?? 50), nextCursor: null };
  },
  async getArtifact(artifactId) {
    return requireArtifact(artifactId);
  },
  async readArtifact(input) {
    const artifact = requireArtifact(input.artifactId);
    return {
      artifactId: artifact.id,
      offset: input.offset,
      bytesBase64: input.offset === 0 && input.length > 0 ? "U2ltdWxhdGVkIGFydGlmYWN0" : "",
      nextOffset: null,
      contentHash: artifact.contentHash,
    };
  },
  async publishArtifact() {
    throw new Error(desktopRuntimeRequired);
  },
  async downloadArtifact() {
    throw new Error(desktopRuntimeRequired);
  },
  async startDelegation(input) {
    const id = `web-attempt-${nextAttemptId++}`;
    const attempt: DelegationAttemptSummary = {
      id,
      delegationId: `web-delegation-${nextAttemptId - 1}`,
      provider: input.provider,
      mode: input.mode,
      status: "failed",
      baseCommit: "",
      changeSetArtifactId: null,
      createdAt: simulatedAt,
      completedAt: simulatedAt,
    };
    attempts.set(id, attempt);
    return structuredClone(attempt);
  },
  async listDelegationAttempts() {
    return [...attempts.values()].map((attempt) => structuredClone(attempt));
  },
  async getDelegationReport(attemptId) {
    const attempt = structuredClone(requireAttempt(attemptId));
    return {
      attempt,
      outcome: "failed",
      summary: desktopRuntimeRequired,
      hostEvidence: [],
      providerClaims: [],
      warnings: [desktopRuntimeRequired],
    };
  },
  async getChangeSetReview(artifactId) {
    const artifact = requireArtifact(artifactId);
    return {
      artifact,
      repositoryIdentity: "",
      baseCommit: "",
      diffHash: "",
      files: [],
      diffText: "",
      riskClassification: "unavailable",
      applyable: false,
    };
  },
  async applyDelegationChanges() {
    throw new Error(desktopRuntimeRequired);
  },
  async getDelegationRecovery(operationId) {
    requireOperation(operationId);
    return { operationId, state: "manual_recovery", capsuleReference: null };
  },
  async getBrowserHandoff(operationId) {
    const handoff = handoffs.get(operationId);
    if (!handoff) throw new Error(desktopRuntimeRequired);
    return structuredClone(handoff);
  },
  async beginBrowserHandoff() {
    throw new Error(desktopRuntimeRequired);
  },
  async resumeBrowserAutomation() {
    throw new Error(desktopRuntimeRequired);
  },
};
