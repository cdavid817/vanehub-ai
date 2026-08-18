import type { Session, WorkflowState } from "../types/agent";
import type { SessionStateEvent } from "./agent-service";
import { mockWorkflowState } from "./mock-agent-data";
import { nowIso } from "./web-mock-clock";

// These bindings are owned here and never exported. An exported mutable binding re-imported from
// two modules gives two divergent copies of the mock world, which surfaces as one UI panel showing
// stale data while another shows fresh. Callers reach the state through the accessors below, which
// read the live binding on every call and so cannot fork.
let sessions: Session[] = [];
let activeSessionId: string | null = null;
let workflowState: WorkflowState = { ...mockWorkflowState };
let nextSessionId = 1;
let nextSeatId = 1;
const sessionEventSubscribers = new Set<(event: SessionStateEvent) => void>();

/** Returns the live array, matching what a direct read of the binding returned. */
export function listWebSessions(): Session[] {
  return sessions;
}

export function replaceWebSessions(next: Session[]): void {
  sessions = next;
}

export function prependWebSession(session: Session): void {
  sessions = [session, ...sessions];
}

export function findWebSession(sessionId: string): Session {
  const session = sessions.find((candidate) => candidate.id === sessionId);
  if (!session) {
    throw new Error(`Session not found: ${sessionId}`);
  }
  return session;
}

export function getWebActiveSessionId(): string | null {
  return activeSessionId;
}

export function setWebActiveSessionId(sessionId: string | null): void {
  activeSessionId = sessionId;
}

export function getWebWorkflowState(): WorkflowState {
  return workflowState;
}

export function setWebWorkflowState(next: WorkflowState): void {
  workflowState = next;
}

/** Every call site reads and advances in one step, so the accessor does too. */
export function nextWebSessionSequence(): number {
  const sequence = nextSessionId;
  nextSessionId += 1;
  return sequence;
}

export function createWebSeatId(): string {
  const seatId = `web-seat-${nextSeatId}`;
  nextSeatId += 1;
  return seatId;
}

export function emitWebSessionEvent(event: SessionStateEvent): void {
  sessionEventSubscribers.forEach((handler) => handler(event));
}

export function subscribeWebSessionStateEvents(
  handler: (event: SessionStateEvent) => void,
): () => void {
  sessionEventSubscribers.add(handler);
  return () => {
    sessionEventSubscribers.delete(handler);
  };
}

export function sortWebSessions(items: Session[]): Session[] {
  return [...items].sort((left, right) => {
    if (left.pinned !== right.pinned) return left.pinned ? -1 : 1;
    if (left.archived !== right.archived) return left.archived ? 1 : -1;
    return right.updatedAt.localeCompare(left.updatedAt);
  });
}

/** The workflow snapshot follows the active session, so the write path stays with the bindings. */
export function updateWebSession(sessionId: string, updates: Partial<Session>): Session {
  const timestamp = nowIso();
  const sessionIndex = sessions.findIndex((session) => session.id === sessionId);
  if (sessionIndex === -1) {
    throw new Error(`Session not found: ${sessionId}`);
  }
  const updatedSession: Session = { ...sessions[sessionIndex], ...updates, updatedAt: timestamp };
  sessions = sessions.map((session, index) => (index === sessionIndex ? updatedSession : session));
  if (activeSessionId === sessionId) {
    workflowState = {
      ...workflowState,
      activeAgentId: updatedSession.agentId,
      activeInteractionMode: updatedSession.interactionMode,
      lifecycleState: updatedSession.lifecycleState,
    };
  }
  return updatedSession;
}
