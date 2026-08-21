import { describe, expect, it } from "vitest";
import type { SessionSeat } from "../types/agent";
import type { ChatMessage } from "../types/chat";
import { webRoutedSeatId } from "./web-session-seat-client";

function seat(seatId: string, roleName: string | null, agentName: string): SessionSeat {
  return {
    seatId,
    agentId: agentName.toLowerCase().replaceAll(" ", "-"),
    roleId: roleName ? `role-${seatId}` : null,
    roleSnapshot: {
      roleName,
      avatar: "🤖",
      color: "#336699",
      responsibility: null,
      agentName,
      modelFamily: "unknown",
      crossFamilyReviewer: false,
    },
  };
}

function assistantFrom(seatId: string): ChatMessage {
  return {
    id: `m-${seatId}`,
    sessionId: "s1",
    role: "assistant",
    speakerSeatId: seatId,
    content: "好的",
    status: "completed",
    createdAt: "2026-08-06T00:00:00Z",
    updatedAt: "2026-08-06T00:00:00Z",
    sessionSequence: 1,
    executionRunId: null,
  };
}

const seats = [seat("seat-1", "架构师", "Claude Code"), seat("seat-2", "实现者", "Codex CLI")];

describe("webRoutedSeatId", () => {
  it("routes a line-leading mention to the named seat", () => {
    expect(webRoutedSeatId(seats, [], "@实现者 写一版")).toBe("seat-2");
  });

  it("keeps an unaddressed message with the seat that last spoke", () => {
    expect(webRoutedSeatId(seats, [assistantFrom("seat-2")], "继续")).toBe("seat-2");
  });

  it("falls back to the first seat when nobody has spoken", () => {
    expect(webRoutedSeatId(seats, [], "开始吧")).toBe("seat-1");
  });

  it("does not let a mid-line mention address anyone", () => {
    expect(webRoutedSeatId(seats, [assistantFrom("seat-1")], "让 @实现者 看一下")).toBe("seat-1");
  });

  it("returns undefined for a single-seat session", () => {
    expect(webRoutedSeatId([seats[0]], [], "@架构师 开始")).toBeUndefined();
  });

  /** A departed seat's handle is a dead letter; the message must still land on someone seated. */
  it("routes a mention of a departed seat back to the first seat", () => {
    const remaining = [seats[0], seat("seat-3", "代码审查", "Claude Code")];
    expect(webRoutedSeatId(remaining, [], "@实现者 写一版")).toBe("seat-1");
  });
});
