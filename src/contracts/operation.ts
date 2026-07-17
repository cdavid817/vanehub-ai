export type OperationKind = "sdk" | "mcp" | "agent" | "workspace" | "extension";

export type OperationStatus = "queued" | "running" | "succeeded" | "failed" | "cancelled";

export interface OperationLogEntry {
  operationId: string;
  line: string;
  timestamp: string;
}

export interface OperationTask {
  id: string;
  kind: OperationKind;
  status: OperationStatus;
  relatedEntityId?: string | null;
  message?: string | null;
  logs: OperationLogEntry[];
  result?: Record<string, unknown> | null;
  error?: string | null;
  createdAt: string;
  updatedAt: string;
}
