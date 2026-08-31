import type { SessionRecoverySummary } from "../services/agent-service";
import { sessionSendBlockReason, type SessionSendBlockReason } from "../services/session-admission";
import type { TurnStatus } from "../components/chat/TurnStatusBar";
import type { Session, SessionLifecycleState } from "../types/agent";

/**
 * What the reader is allowed to do right now, in the exact priority order the existing
 * `submit()`/`stop()` guards in use-main-layout-model.ts already apply — this consolidates that
 * logic into data instead of duplicating the checks in every consumer that needs to know "is Send
 * or Stop the right button", not a new policy. A session whose active execution run exists but
 * has not yet produced a streaming message (`isStreaming` false while a run id is still set) is a
 * real, pre-existing gap: neither `send` nor `stop` is offered, faithfully reported as `blocked`
 * with reason `"active-execution"` rather than smoothed over.
 */
export type SessionPrimaryAction =
  | { kind: "none" }
  | { kind: "send" }
  | { kind: "stop" }
  | { kind: "recover"; acknowledging: boolean }
  | { kind: "blocked"; reason: SessionSendBlockReason };

export interface SessionPresentation {
  session: Session | null;
  /** `"no-session"` is not a `SessionLifecycleState` value — it is what "nothing is open" is. */
  lifecycle: SessionLifecycleState | "no-session";
  activeExecution: {
    isStreaming: boolean;
    isSending: boolean;
  };
  /** `null` when nobody currently holds the turn (no session, or nothing streaming/waiting). */
  participantTurn: TurnStatus | null;
  recovery: {
    status: Session["recoveryStatus"];
    summary: SessionRecoverySummary | null;
    acknowledging: boolean;
  } | null;
  messageState: {
    count: number;
    /** True once more messages exist than the current page loaded — see `loadEarlier`. */
    partial: boolean;
    loading: boolean;
  };
  primaryAction: SessionPrimaryAction;
}

export interface DeriveSessionPresentationInput {
  session: Session | null;
  isStreaming: boolean;
  isSending: boolean;
  turnStatus: TurnStatus | null;
  recoverySummary: SessionRecoverySummary | null;
  acknowledgingRecovery: boolean;
  messageCount: number;
  messagesPartial: boolean;
  messagesLoading: boolean;
}

function derivePrimaryAction(
  session: Session | null,
  isStreaming: boolean,
  acknowledgingRecovery: boolean,
): SessionPrimaryAction {
  if (!session) return { kind: "none" };
  if (isStreaming) return { kind: "stop" };
  const blockReason = sessionSendBlockReason(session);
  if (blockReason === "recovery") return { kind: "recover", acknowledging: acknowledgingRecovery };
  if (blockReason !== null) return { kind: "blocked", reason: blockReason };
  return { kind: "send" };
}

/**
 * Consolidates the lifecycle/execution/turn/recovery/message-state/primary-action view task 10.1
 * asks for out of `MainLayoutModel`'s ~30 separately-named fields — a pure function, not a hook,
 * so header/composer/primary-action consumers can each derive their own slice without depending
 * on the whole model shape or re-deriving `sessionSendBlockReason`/`isStreaming` priority themselves.
 */
export function deriveSessionPresentation(input: DeriveSessionPresentationInput): SessionPresentation {
  const { session, isStreaming, isSending, turnStatus, recoverySummary, acknowledgingRecovery, messageCount, messagesPartial, messagesLoading } = input;
  return {
    session,
    lifecycle: session?.lifecycleState ?? "no-session",
    activeExecution: { isStreaming, isSending },
    participantTurn: turnStatus,
    recovery: session && session.recoveryStatus !== "clean"
      ? { status: session.recoveryStatus, summary: recoverySummary, acknowledging: acknowledgingRecovery }
      : null,
    messageState: { count: messageCount, loading: messagesLoading, partial: messagesPartial },
    primaryAction: derivePrimaryAction(session, isStreaming, acknowledgingRecovery),
  };
}
