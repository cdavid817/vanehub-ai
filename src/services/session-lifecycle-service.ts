import type { CreateSessionInput, Session, UpdateSessionSeatsInput } from "../types/agent";
import type { OperationTask } from "../types/operation";

export interface SessionLifecycleService {
  createSession(input: CreateSessionInput): Promise<OperationTask>;
  deleteSession(sessionId: string): Promise<void>;
  switchSession(sessionId: string): Promise<Session>;
  renameSession(sessionId: string, title: string): Promise<Session>;
  pinSession(sessionId: string): Promise<Session>;
  unpinSession(sessionId: string): Promise<Session>;
  archiveSession(sessionId: string): Promise<Session>;
  unarchiveSession(sessionId: string): Promise<Session>;
}

export interface SessionSeatService {
  updateSessionSeats(input: UpdateSessionSeatsInput): Promise<Session>;
  rebindRemoteSessionSshConnection(sessionId: string, connectionId: string): Promise<Session>;
}
