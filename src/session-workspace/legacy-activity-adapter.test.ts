import { readFileSync } from "node:fs";
import { describe, expect, it } from "vitest";
import { evidenceSessionIdSchema } from "../contracts/session-workspace-evidence-ids";
import type { ChatMessage, ToolUseBlock } from "../types/chat";
import {
  legacyActivityRecords,
  legacyCoverage,
  LEGACY_ACTIVITY_REASON,
  LEGACY_WINDOW_PARTIAL_REASON,
} from "./legacy-activity-adapter";

const sessionId = evidenceSessionIdSchema.parse("session-a");

function message(overrides: Partial<ChatMessage> & { id: string }): ChatMessage {
  return {
    sessionId: "session-a",
    role: "assistant",
    content: "",
    status: "completed",
    createdAt: "2026-08-23T10:00:00.000Z",
    updatedAt: "2026-08-23T10:00:01.000Z",
    sessionSequence: 1,
    executionRunId: null,
    ...overrides,
  };
}

function tool(id: string, name: string, status: ToolUseBlock["status"]): ToolUseBlock {
  return { id, name, status };
}

describe("legacy activity adapter", () => {
  it("projects loaded tool activity newest first", () => {
    const { records } = legacyActivityRecords({
      messagesPartial: false,
      sessionId,
      messages: [
        message({ id: "m1", toolUse: [tool("t1", "read_file", "completed")] }),
        message({ id: "m2", toolUse: [tool("t2", "write_file", "failed")] }),
      ],
    });

    expect(records.map((record) => record.label)).toEqual(["write_file", "read_file"]);
    expect(records.map((record) => record.messageId)).toEqual(["m2", "m1"]);
    expect(records.map((record) => record.status)).toEqual(["failed", "succeeded"]);
  });

  it("marks every record inferred and sourced from message history", () => {
    const { records } = legacyActivityRecords({
      messagesPartial: false,
      sessionId,
      messages: [message({ id: "m1", toolUse: [tool("t1", "read_file", "completed")] })],
    });

    // A `toolUse` block records what an assistant said it was doing. Reporting that as native
    // would make it indistinguishable afterwards from work the runtime actually witnessed.
    expect(records[0].fidelity).toBe("inferred");
    expect(records[0].source).toBe("message-history");
    expect(records[0].kind).toBe("legacy");
  });

  it("leaves absent every field a message cannot support", () => {
    const { records } = legacyActivityRecords({
      messagesPartial: false,
      sessionId,
      messages: [message({ id: "m1", toolUse: [tool("t1", "read_file", "completed")] })],
    });
    const record: Record<string, unknown> = { ...records[0] };

    // A message's `createdAt` is when the message was created, not when the tool started or
    // finished, and a duration derived from it would be arithmetic presented as an observation.
    for (const field of ["startedAt", "endedAt", "durationMs", "commandId", "exitCode", "traceId"]) {
      expect(record[field], field).toBeUndefined();
    }
  });

  it("passes through the attribution the message itself carries", () => {
    const { records } = legacyActivityRecords({
      messagesPartial: false,
      sessionId,
      messages: [
        message({
          id: "m1",
          executionRunId: "run-1",
          speakerSeatId: "seat-1",
          toolUse: [tool("t1", "read_file", "completed")],
        }),
      ],
    });

    // Reported, not guessed: the message states both.
    expect(records[0].runId).toBe("run-1");
    expect(records[0].seatId).toBe("seat-1");
  });

  it("narrows to one seat without attributing unattributed messages to it", () => {
    const { records } = legacyActivityRecords({
      messagesPartial: false,
      seatId: "seat-1",
      sessionId,
      messages: [
        message({ id: "m1", speakerSeatId: "seat-1", toolUse: [tool("t1", "a", "completed")] }),
        message({ id: "m2", speakerSeatId: "seat-2", toolUse: [tool("t2", "b", "completed")] }),
        message({ id: "m3", toolUse: [tool("t3", "c", "completed")] }),
      ],
    });

    // A message written before speaker attribution existed carries no seat, so it is not shown as
    // this seat's work.
    expect(records.map((record) => record.label)).toEqual(["a"]);
  });

  it("never reports complete coverage, even for a window it believes is whole", () => {
    // Compaction removes messages without leaving anything a reader could count, so "I have all
    // of them" is a claim this side of the boundary cannot make.
    expect(legacyCoverage(false)).toEqual({
      state: "partial",
      reasonCodes: [LEGACY_ACTIVITY_REASON],
      truncated: false,
    });
    expect(legacyCoverage(true)).toEqual({
      state: "partial",
      reasonCodes: [LEGACY_ACTIVITY_REASON, LEGACY_WINDOW_PARTIAL_REASON],
      truncated: true,
    });
  });

  it("gives each record the same coverage the projection reports", () => {
    const { coverage, records } = legacyActivityRecords({
      messagesPartial: true,
      sessionId,
      messages: [message({ id: "m1", toolUse: [tool("t1", "read_file", "running")] })],
    });

    expect(coverage.state).toBe("partial");
    expect(records[0].coverage).toEqual(coverage);
  });

  it("maps a waiting tool to a queued record rather than inventing a state", () => {
    const { records } = legacyActivityRecords({
      messagesPartial: false,
      sessionId,
      messages: [
        message({
          id: "m1",
          toolUse: [
            tool("t1", "a", "awaiting_approval"),
            tool("t2", "b", "awaiting_input"),
            tool("t3", "c", "pending"),
            tool("t4", "d", "cancelled"),
          ],
        }),
      ],
    });

    expect(records.map((record) => record.status)).toEqual([
      "queued",
      "queued",
      "queued",
      "cancelled",
    ]);
  });

  it("reaches no service at all", () => {
    const source = readFileSync("src/session-workspace/legacy-activity-adapter.ts", "utf8");

    // The adapter is a pure read of what the caller already holds. A write here would put a
    // message's claim into the journal beside events the runtime witnessed, and afterwards
    // nothing could tell them apart.
    expect(source).not.toContain("agentService");
    expect(source).not.toContain("runtime-agent-client");
    expect(source).not.toContain("invoke");
  });
});
