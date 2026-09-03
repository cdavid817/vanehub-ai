export type OperationKind = "sdk" | "mcp" | "agent" | "workspace" | "extension" | "cli";

export type OperationStatus = "queued" | "running" | "succeeded" | "failed" | "cancelled";

export interface OperationLogEntry {
  operationId: string;
  line: string;
  timestamp: string;
}

export interface OperationTask {
  id: string;
  executionRunId?: string | null;
  traceId?: string | null;
  kind: OperationKind;
  status: OperationStatus;
  relatedEntityId?: string | null;
  message?: string | null;
  logs: OperationLogEntry[];
  result?: Record<string, unknown> | null;
  error?: string | null;
  createdAt: string;
  updatedAt: string;
  /** Descriptive stage. `status` stays authoritative for whether the work finished. */
  phase?: string | null;
  completedUnits?: number | null;
  totalUnits?: number | null;
  /**
   * Whether cancellation can be requested right now. Absent means the operation never declared
   * one way or the other, and cancelling never implies an already-applied external effect was
   * undone.
   */
  cancellable?: boolean | null;
}
