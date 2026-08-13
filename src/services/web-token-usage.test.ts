import { describe, expect, it } from "vitest";
import { queryWebTokenUsageDetails, queryWebTokenUsageSummary } from "./web-token-usage";

describe("Web Token usage ledger fixtures", () => {
  it("keeps quality, purpose, multi-call, and unit totals separate", () => {
    const summary = queryWebTokenUsageSummary({});

    expect(summary.schemaVersion).toBe(1);
    expect(summary.totals.reported.headlineTotal).toBe(480);
    expect(summary.totals.reportedDerived.headlineTotal).toBe(75);
    expect(summary.totals.estimated.headlineTotal).toBe(900);
    expect(summary.totals.estimated.unit).toBe("characters");
    expect(summary.userResponse.reported.headlineTotal).toBe(440);
    expect(summary.internal.reported.headlineTotal).toBe(40);
    expect(summary.counts).toEqual({ calls: 6, generations: 3, sessions: 2 });
    expect(summary.daily[0]?.localDate).toBe("2026-08-10");
  });

  it("applies the same dimensions to totals and breakdowns", () => {
    const failedOnePiece = queryWebTokenUsageSummary({
      agentId: "onepiece",
      status: "failed",
    });

    expect(failedOnePiece.counts.calls).toBe(1);
    expect(failedOnePiece.totals.reported.headlineTotal).toBe(90);
    expect(failedOnePiece.breakdowns.find(({ dimension }) => dimension === "purpose")?.entries)
      .toMatchObject([{ key: "tool-continuation" }]);

    const unknownProviders = queryWebTokenUsageSummary({ agentId: "onepiece" })
      .breakdowns.find(({ dimension }) => dimension === "provider")?.entries;
    expect(unknownProviders?.some(({ key }) => key === "unknown")).toBe(true);
  });

  it("returns bounded cursor pages and an isolated empty range", () => {
    const first = queryWebTokenUsageDetails({ limit: 2 });
    expect(first.invocations).toHaveLength(2);
    expect(first.observations).toHaveLength(2);
    expect(first.nextCursor).toBe("web-inv-terminal");

    const second = queryWebTokenUsageDetails({ afterId: first.nextCursor ?? undefined, limit: 10 });
    expect(second.invocations).toHaveLength(4);
    expect(second.nextCursor).toBeNull();

    const empty = queryWebTokenUsageSummary({ rangeStart: "2026-08-11T00:00:00.000Z" });
    expect(empty.counts.calls).toBe(0);
    expect(empty.daily).toEqual([]);
    expect(empty.totals.reported.headlineTotal).toBe(0);
  });

  it("compares offset ranges as instants and emits a local-calendar date", () => {
    const summary = queryWebTokenUsageSummary({
      rangeStart: "2026-08-10T17:00:00+08:00",
      rangeEnd: "2026-08-10T19:00:00+08:00",
    });
    const event = new Date("2026-08-10T10:00:00.000Z");
    const localDate = [
      event.getFullYear(),
      String(event.getMonth() + 1).padStart(2, "0"),
      String(event.getDate()).padStart(2, "0"),
    ].join("-");

    expect(summary.counts.calls).toBe(6);
    expect(summary.daily).toHaveLength(1);
    expect(summary.daily[0]?.localDate).toBe(localDate);
  });
});
