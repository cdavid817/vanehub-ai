import type { AgentRun } from "../types/agent-run";
import type { AgentRunnerDescriptor, AgentRunnerSelection } from "../types/agent-runner";
import { findWebSshConnection } from "./web-ssh-connection-client";
import { findWebSession } from "./web-session-state";

export function webRunnerDescriptors(sessionId: string, agentId: string): AgentRunnerDescriptor[] {
  const session = findWebSession(sessionId);
  if (session.agentId !== agentId) throw new Error("runner_invalid_selection");
  const descriptors: AgentRunnerDescriptor[] = [{
    selection: { kind: "local" },
    label: "Local",
    hostLabel: "This device",
    available: true,
    unavailableReason: null,
    simulated: true,
    capabilities: { interactiveInput: true, pty: false, cancellation: true, inspection: true, recovery: "none" },
  }];
  const connectionId = session.remoteSshConnectionId;
  const revision = session.remoteSshConnectionRevision;
  if (connectionId && revision && session.remoteWorkspace) {
    const connection = findWebSshConnection(connectionId);
    const credentialConfigured = connection?.authMode === "password" ? connection.hasPassword : Boolean(connection?.keyPath);
    const available = Boolean(connection
      && connection.revision === revision
      && connection.host === session.remoteWorkspace.host
      && connection.port === (session.remoteWorkspace.port ?? 22)
      && connection.user === session.remoteWorkspace.user
      && connection.hostTrust
      && credentialConfigured);
    descriptors.push({
      selection: { kind: "ssh", targetId: connectionId, targetRevision: revision },
      label: session.remoteWorkspace.displayName,
      hostLabel: session.remoteWorkspace.host,
      available,
      unavailableReason: available ? null : "ssh_authority_unavailable",
      simulated: true,
      capabilities: { interactiveInput: true, pty: true, cancellation: true, inspection: true, recovery: "inspect_only" },
    });
  }
  descriptors.push(
    {
      selection: { kind: "docker" }, label: "Docker / Sandbox", hostLabel: null,
      available: false, unavailableReason: "runner_not_implemented", simulated: true,
      capabilities: { interactiveInput: false, pty: false, cancellation: false, inspection: false, recovery: "none" },
    },
    {
      selection: { kind: "cloud" }, label: "Cloud", hostLabel: null,
      available: false, unavailableReason: "runner_not_implemented", simulated: true,
      capabilities: { interactiveInput: false, pty: false, cancellation: false, inspection: false, recovery: "none" },
    },
  );
  return descriptors;
}

export function selectWebRunner(sessionId: string, agentId: string, requested?: AgentRunnerSelection): AgentRunnerDescriptor {
  const selection = requested ?? { kind: "local" };
  const descriptor = webRunnerDescriptors(sessionId, agentId).find((candidate) =>
    candidate.selection.kind === selection.kind
    && (candidate.selection.targetId ?? null) === (selection.targetId ?? null)
    && (candidate.selection.targetRevision ?? null) === (selection.targetRevision ?? null));
  if (!descriptor) throw new Error("runner_invalid_selection");
  if (!descriptor.available) throw new Error("runner_unsupported_capability");
  return descriptor;
}

export function webRunRunner(descriptor: AgentRunnerDescriptor): NonNullable<AgentRun["runner"]> {
  const targetId = descriptor.selection.targetId ?? "local";
  const targetRevision = descriptor.selection.targetRevision ?? null;
  return {
    kind: descriptor.selection.kind as "local" | "ssh",
    targetId,
    targetRevision,
    label: descriptor.label,
    hostLabel: descriptor.hostLabel,
    recovery: descriptor.capabilities.recovery,
    capabilityWitness: `web-simulated:${descriptor.selection.kind}`,
    authorityWitness: `web-simulated:${targetId}:${targetRevision ?? "none"}`,
    recoveryReference: null,
  };
}
