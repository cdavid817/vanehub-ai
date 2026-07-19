import { describe, expect, it } from "vitest";
import { computeNextScheduledRun, validateScheduledTaskFrequency } from "./scheduled-task-recurrence";

describe("scheduled task recurrence", () => {
  it("computes interval schedules from the current time", () => {
    const from = new Date("2026-07-19T01:00:00.000Z");

    expect(computeNextScheduledRun({ kind: "minutes", interval: 15 }, from)).toBe("2026-07-19T01:15:00.000Z");
    expect(computeNextScheduledRun({ kind: "hours", interval: 2 }, from)).toBe("2026-07-19T03:00:00.000Z");
  });

  it("rejects invalid recurrence values", () => {
    expect(() => validateScheduledTaskFrequency({ kind: "minutes", interval: 0 })).toThrow();
    expect(() => validateScheduledTaskFrequency({ kind: "weekly", weekday: 7, timeOfDay: "09:00" })).toThrow();
    expect(() => validateScheduledTaskFrequency({ kind: "monthly", dayOfMonth: 32, timeOfDay: "09:00" })).toThrow();
  });
});
