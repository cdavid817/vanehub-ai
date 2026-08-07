import { describe, expect, it } from "vitest";
import { turnStatusFromEvent, waitedMinutes } from "./turn-status";

describe("turnStatusFromEvent", () => {
  it("names the seat currently speaking and its chain position", () => {
    expect(
      turnStatusFromEvent({
        kind: "agent",
        seatIndex: 1,
        mention: "代码审查",
        depth: 3,
        maxDepth: 15,
      }),
    ).toEqual({ kind: "agent", holderName: "代码审查", depth: 3, maxDepth: 15 });
  });

  it("names who asked the human and when the wait began", () => {
    const status = turnStatusFromEvent({
      kind: "waiting_human",
      seatIndex: 0,
      mention: "架构师",
      since: "2026-08-07T10:00:00Z",
    });
    expect(status).toMatchObject({ kind: "waiting-human", requesterName: "架构师" });
  });

  it("names who finished the round", () => {
    expect(
      turnStatusFromEvent({ kind: "round_complete", seatIndex: 0, mention: "架构师" }),
    ).toEqual({ kind: "round-complete", finisherName: "架构师" });
  });
});

describe("waitedMinutes", () => {
  it("counts whole minutes since the wait began", () => {
    expect(waitedMinutes("2026-08-07T10:00:00Z", new Date("2026-08-07T10:07:30Z"))).toBe(7);
  });

  // A wait that just started reads as zero, not as a negative or a blank.
  it("counts a wait that just began as zero", () => {
    expect(waitedMinutes("2026-08-07T10:00:00Z", new Date("2026-08-07T10:00:00Z"))).toBe(0);
  });

  /// A clock that disagrees with the backend's must not show a negative wait.
  it("never counts backwards", () => {
    expect(waitedMinutes("2026-08-07T10:05:00Z", new Date("2026-08-07T10:00:00Z"))).toBe(0);
  });

  it("counts an unreadable timestamp as zero", () => {
    expect(waitedMinutes("not a time", new Date("2026-08-07T10:00:00Z"))).toBe(0);
  });
});
