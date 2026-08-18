export type AgentRunnerKind = "local" | "ssh" | "docker" | "cloud";
export type AgentRunnerRecovery = "none" | "inspect_only" | "reattach";

export interface AgentRunnerSelection {
  kind: AgentRunnerKind;
  targetId?: string | null;
  targetRevision?: number | null;
}

export interface AgentRunnerCapabilities {
  interactiveInput: boolean;
  pty: boolean;
  cancellation: boolean;
  inspection: boolean;
  recovery: AgentRunnerRecovery;
}

export interface AgentRunnerDescriptor {
  selection: AgentRunnerSelection;
  label: string;
  hostLabel: string | null;
  available: boolean;
  unavailableReason: string | null;
  simulated: boolean;
  capabilities: AgentRunnerCapabilities;
}

export type AgentRunnerErrorCode =
  | "runner_unsupported_capability" | "runner_invalid_selection" | "runner_authority_stale"
  | "runner_permission_denied" | "runner_invalid_launch" | "runner_preparation_failed"
  | "runner_spawn_failed" | "runner_input_failed" | "runner_disconnected"
  | "runner_reconnect_exhausted" | "runner_cancellation_failed" | "runner_inspection_failed"
  | "runner_cleanup_failed" | "runner_resource_exhausted";

export interface AgentRunnerSafeError {
  code: AgentRunnerErrorCode;
  message: string;
}
