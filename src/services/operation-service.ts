import type { OperationTask } from "../types/operation";

export interface OperationService {
  listOperations(): Promise<OperationTask[]>;
  getOperationStatus(operationId: string): Promise<OperationTask>;
}
