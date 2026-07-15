import type { OperationTask } from "../types/operation";
import type { OperationService } from "./operation-service";

const mockOperations: OperationTask[] = [];

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
