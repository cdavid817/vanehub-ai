import { describe, expect, it } from "vitest";
import type { ChatMessage } from "../types/chat";
import type { EvidenceSessionId } from "../types/session-workspace-evidence";
import { aggregateSessionReport } from "./report-utils";
import { emptySessionRunReport } from "./report-test-fixtures";

/**
 * What the old message aggregation and the new backend report disagree about, held as tests.
 *
 * Retained deliberately. The Report tab no longer calls `aggregateSessionReport`, so the divergences
 * below are no longer visible anywhere in the product — and they are the reason the replacement
 * happened, which makes them exactly the thing that gets forgotten and then "fixed" by making the
 * backend match the old numbers.
 *
 * Each case states a figure the two produce differently and which one is right about what.
 */

const SESSION = "session-1" as EvidenceSessionId;

function message(overrides: Partial<ChatMessage> = {}): ChatMessage {
  return {
    id: "message-1",
    sessionId: "session-1",
    role: "assistant",
    content: "hello",
    status: "completed",
    toolUse: [],
    tokenUsage: { input: 10, output: 5 },
    createdAt: "2026-08-25T10:00:00.000Z",
    updatedAt: "2026-08-25T10:00:01.000Z",
    sessionSequence: 1,
    executionRunId: null,
    ...overrides,
  };
}

describe("legacy report parity", () => {
  /**
   * The divergence the replacement exists for.
   *
   * The legacy figure is a function of how many messages are mounted, so scrolling changed it. The
   * backend's is a function of what the session did. They are not two estimates of one quantity;
   * they are two different quantities, and only one of them answers "what did this session use".
   */
  it("legacy usage tracks the mounted messages while the backend tracks the session", () => {
    const mounted = [message({ id: "a" }), message({ id: "b" })];
    const paged = [...mounted, message({ id: "c" })];

    expect(aggregateSessionReport(mounted).reportedInputTokens).toBe(20);
    expect(aggregateSessionReport(paged).reportedInputTokens).toBe(30);

    // The backend answer does not move: nothing about the report's inputs changed when a third
    // message was paged in.
    const report = emptySessionRunReport(SESSION);
    expect(report.usage.reportedInputTokens).toBeUndefined();
  });

  /**
   * The legacy report counted messages; the backend counts runs.
   *
   * A run can span many messages and a message can belong to none, so the two never agreed and were
   * never supposed to. Reading `runCount` as "how many turns" is the mistake this records.
   */
  it("legacy status counts are per message and the backend's are per run", () => {
    const legacy = aggregateSessionReport([
      message({ id: "a", status: "completed" }),
      message({ id: "b", status: "failed" }),
    ]);

    expect(legacy.statusCounts.completed).toBe(1);
    expect(legacy.statusCounts.failed).toBe(1);
    expect(legacy.messageCount).toBe(2);

    // No `messageCount` exists on the backend report at all, which is the point: the number a
    // reader wants is runs, and offering both would invite them to be compared.
    const report = emptySessionRunReport(SESSION);
    expect(report.overview.runCount).toBe(0);
    expect("messageCount" in report.overview).toBe(false);
  });

  /**
   * The legacy aggregation could not tell "no tools were used" from "the tool calls were not in the
   * mounted window". Both produced an empty ranking with no accompanying claim.
   */
  it("legacy emptiness carries no coverage and the backend's does", () => {
    expect(aggregateSessionReport([]).toolRanking).toEqual([]);

    const report = emptySessionRunReport(SESSION);
    expect(report.tools).toEqual([]);
    // The same empty list, with the one thing the legacy one could never carry: a statement about
    // whether the emptiness is a fact or a gap.
    expect(report.coverage.sections.tools.state).toBe("complete");
  });

  /**
   * The legacy report summed reported tokens and estimated characters into neighbouring metrics
   * with no unit between them. The backend keeps the three qualities apart all the way out.
   */
  it("legacy estimates and reported figures sit in the same shape and the backend's do not", () => {
    const legacy = aggregateSessionReport([
      message({ id: "a", role: "user", content: "abcd", tokenUsage: undefined }),
    ]);

    // Characters, presented beside token counts as though they were comparable.
    expect(legacy.estimatedInputCharacters).toBe(4);
    expect(legacy.reportedInputTokens).toBe(0);

    const report = emptySessionRunReport(SESSION);
    // Absent rather than zero, and named in its own unit.
    expect(report.usage.reportedInputTokens).toBeUndefined();
    expect(report.usage.estimatedCharacters).toBeUndefined();
  });
});
