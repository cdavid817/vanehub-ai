import { describe, expect, it } from "vitest";
import type { SafeAttribute } from "../types/execution-observability";
import { traceSeat } from "./trace-seat";

function attributes(entries: Record<string, string>): Record<string, SafeAttribute> {
  return entries;
}

describe("traceSeat", () => {
  it("reads the seat a span belongs to", () => {
    expect(
      traceSeat(attributes({ "vanehub.seat.index": "1", "vanehub.seat.mention": "代码审查" })),
    ).toEqual({ seatIndex: 1, mention: "代码审查" });
  });

  // A single-Agent session's spans carry no seat, and the trace must render as it does today.
  it("reads nothing from a span with no seat", () => {
    expect(traceSeat(attributes({ "vanehub.agent.id": "claude-code" }))).toBeNull();
  });

  it("reads nothing when the index is not a number", () => {
    expect(traceSeat(attributes({ "vanehub.seat.index": "第一个" }))).toBeNull();
  });

  it("reads a seat with no mention", () => {
    expect(traceSeat(attributes({ "vanehub.seat.index": "0" }))).toEqual({
      seatIndex: 0,
      mention: null,
    });
  });
});
