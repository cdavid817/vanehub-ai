import type {
  ExecuteSessionDeletionInput,
  PreviewSessionDeletionInput,
  RetrySessionDeletionInput,
  SessionDeletionHandle,
  SessionDeletionOperation,
  SessionDeletionPreview,
} from "../types/session-deletion";

/**
 * The confirmed session-deletion flow. Every visible delete entry point goes through this:
 * preview first, then an explicit execution bound to that preview. The legacy
 * `deleteSession(sessionId)` on the lifecycle service is keep-only and runs the same native
 * arbitration; nothing in the UI calls it directly.
 */
export interface SessionDeletionService {
  previewSessionDeletion(input: PreviewSessionDeletionInput): Promise<SessionDeletionPreview>;
  executeSessionDeletion(input: ExecuteSessionDeletionInput): Promise<SessionDeletionHandle>;
  getSessionDeletionOperation(operationId: string): Promise<SessionDeletionOperation>;
  listPendingSessionDeletions(): Promise<SessionDeletionOperation[]>;
  retrySessionDeletion(input: RetrySessionDeletionInput): Promise<SessionDeletionHandle>;
}
