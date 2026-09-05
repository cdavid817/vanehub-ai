import { invoke } from "@tauri-apps/api/core";
import type { SessionDeletionService } from "./session-deletion-service";
import type {
  ExecuteSessionDeletionInput,
  PreviewSessionDeletionInput,
  RetrySessionDeletionInput,
  SessionDeletionHandle,
  SessionDeletionOperation,
  SessionDeletionPreview,
} from "../types/session-deletion";

/** Desktop adapter. Every call names a declared native command; no path ever crosses here. */
export const tauriSessionDeletionClient: SessionDeletionService = {
  previewSessionDeletion(input: PreviewSessionDeletionInput) {
    return invoke<SessionDeletionPreview>("preview_session_deletion", { input });
  },

  executeSessionDeletion(input: ExecuteSessionDeletionInput) {
    return invoke<SessionDeletionHandle>("execute_session_deletion", { input });
  },

  getSessionDeletionOperation(operationId: string) {
    return invoke<SessionDeletionOperation>("get_session_deletion_operation", { operationId });
  },

  listPendingSessionDeletions() {
    return invoke<SessionDeletionOperation[]>("list_pending_session_deletions");
  },

  retrySessionDeletion(input: RetrySessionDeletionInput) {
    return invoke<SessionDeletionHandle>("retry_session_deletion", { input });
  },
};
