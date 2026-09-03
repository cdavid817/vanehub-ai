import {
  evidenceRecordIdSchema,
  evidenceRunIdSchema,
  evidenceSeatIdSchema,
} from "../contracts/session-workspace-evidence-ids";
import type {
  EvidenceSessionId,
  EvidenceStatus,
  LegacyActivityRecord,
  QueryCoverage,
} from "../types/session-workspace-evidence";
import type { ChatMessage, ToolUseBlock } from "../types/chat";

/**
 * Why a message-history projection can never call itself complete.
 *
 * The reader is looking at loaded messages, and the frontend cannot see past two boundaries it
 * does not control: the page it asked for, and whatever context compaction removed before that.
 * Neither leaves a mark a consumer could check, so the honest answer is always "some of it".
 */
export const LEGACY_ACTIVITY_REASON = "legacy_activity_message_history";

/** The loaded window is known to be short of the whole history, not merely unverifiable. */
export const LEGACY_WINDOW_PARTIAL_REASON = "legacy_activity_window_partial";

export interface LegacyActivityInput {
  messages: ChatMessage[];
  sessionId: EvidenceSessionId;
  /** Narrow to one participant, or null for every seat. */
  seatId?: string | null;
  /** Whether the caller already knows its loaded message window is incomplete. */
  messagesPartial: boolean;
}

export interface LegacyActivityProjection {
  records: LegacyActivityRecord[];
  coverage: QueryCoverage;
}

/**
 * What a `message.toolUse` block can honestly be read as.
 *
 * `awaiting_approval` and `awaiting_input` are both work that has not finished and is not running
 * either; `queued` is the closest state the evidence vocabulary has, and inventing a state for
 * them would put a status in the console that no producer ever reports.
 */
function legacyStatus(status: ToolUseBlock["status"]): EvidenceStatus {
  switch (status) {
    case "completed":
      return "succeeded";
    case "failed":
      return "failed";
    case "cancelled":
      return "cancelled";
    case "running":
      return "running";
    case "pending":
    case "awaiting_approval":
    case "awaiting_input":
      return "queued";
  }
}

/**
 * Historical tool activity, read out of loaded chat messages.
 *
 * This is a projection, not evidence. Nothing here is written to the journal: a `toolUse` block
 * records what an assistant said it was doing, and filing that beside events the runtime actually
 * witnessed would make the two indistinguishable afterwards — the journal's whole value is that
 * everything in it was observed.
 *
 * So every record is `inferred`, carries `message-history` as its source, and leaves absent every
 * field the message cannot support. There is no command id, no exit code, no duration, and no
 * timestamps: a message's `createdAt` is when the message was created, not when the tool started
 * or finished, and putting it in `startedAt` would be an observation nobody made. Ordering comes
 * from the message order instead, which is authoritative and needs no invention.
 *
 * The run and the seat are the exceptions. The message carries both as its own attribution, so
 * passing them through reports what the message says rather than guessing what it meant.
 */
export function legacyActivityRecords({
  messages,
  messagesPartial,
  seatId = null,
  sessionId,
}: LegacyActivityInput): LegacyActivityProjection {
  const records: LegacyActivityRecord[] = [];
  // Newest first, matching the native list. The messages arrive oldest first.
  for (let index = messages.length - 1; index >= 0; index -= 1) {
    const message = messages[index];
    if (seatId !== null && message.speakerSeatId !== seatId) continue;
    for (const tool of message.toolUse ?? []) {
      const id = evidenceRecordIdSchema.safeParse(`legacy:${message.id}:${tool.id}`);
      // An id that will not parse cannot be de-duplicated or selected, and a row nothing can
      // address is worse than a row that is absent.
      if (!id.success) continue;
      const runId = evidenceRunIdSchema.safeParse(message.executionRunId);
      const seat = evidenceSeatIdSchema.safeParse(message.speakerSeatId);
      records.push({
        id: id.data,
        kind: "legacy",
        sessionId,
        label: tool.name,
        source: "message-history",
        messageId: message.id,
        status: legacyStatus(tool.status),
        fidelity: "inferred",
        coverage: legacyCoverage(messagesPartial),
        ...(runId.success ? { runId: runId.data } : {}),
        ...(seat.success ? { seatId: seat.data } : {}),
      });
    }
  }
  return { coverage: legacyCoverage(messagesPartial), records };
}

/**
 * Always partial, never complete.
 *
 * A loaded window that happens to hold every message still cannot say so: compaction removes
 * messages without leaving anything a reader could count, so "I have all of them" is a claim this
 * side of the boundary is not in a position to make.
 */
export function legacyCoverage(messagesPartial: boolean): QueryCoverage {
  return {
    state: "partial",
    reasonCodes: messagesPartial
      ? [LEGACY_ACTIVITY_REASON, LEGACY_WINDOW_PARTIAL_REASON]
      : [LEGACY_ACTIVITY_REASON],
    truncated: messagesPartial,
  };
}
