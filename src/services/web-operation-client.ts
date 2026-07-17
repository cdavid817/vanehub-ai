import type { OperationTask } from "../types/operation";
import type { OperationService } from "./operation-service";

let mockOperations: OperationTask[] = [];

function nowIso() {
  return new Date().toISOString();
}

export function registerWebOperation(operation: OperationTask) {
  mockOperations = [operation, ...mockOperations.filter((item) => item.id !== operation.id)];
}

function updateWebOperation(operationId: string, updates: Partial<OperationTask>) {
  mockOperations = mockOperations.map((operation) =>
    operation.id === operationId ? { ...operation, ...updates, updatedAt: nowIso() } : operation,
  );
}

export function createWebMockOperation(input: {
  id: string;
  relatedEntityId: string | null;
  message: string;
  terminalStatus: "succeeded" | "failed";
  error: string | null;
  result?: Record<string, unknown> | null;
}): OperationTask {
  const timestamp = nowIso();
  const operation: OperationTask = {
    id: input.id,
    kind: "agent",
    status: "queued",
    relatedEntityId: input.relatedEntityId,
    message: input.message,
    logs: [{ operationId: input.id, line: input.message, timestamp }],
    result: null,
    error: null,
    createdAt: timestamp,
    updatedAt: timestamp,
  };
  registerWebOperation(operation);
  setTimeout(() => {
    updateWebOperation(input.id, {
      status: "running",
      logs: [
        ...operation.logs,
        {
          operationId: input.id,
          line: input.message,
          timestamp: nowIso(),
        },
      ],
    });
  }, 50);
  setTimeout(() => {
    const current = mockOperations.find((item) => item.id === input.id);
    updateWebOperation(input.id, {
      status: input.terminalStatus,
      result: input.result ?? null,
      error: input.error,
      logs: [
        ...(current?.logs ?? operation.logs),
        {
          operationId: input.id,
          line: input.error ?? input.message,
          timestamp: nowIso(),
        },
      ],
    });
  }, 900);
  return operation;
}

export const webOperationClient: OperationService = {
  async listOperations() {
    return mockOperations;
  },

  async getOperationStatus(operationId: string) {
    const operation = mockOperations.find((item) => item.id === operationId);
    if (!operation) {
      throw new Error(`operation not found: ${operationId}`);
    }
    return operation;
  },
};
