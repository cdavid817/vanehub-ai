import { describe, expect, it } from "vitest";
import { computeNextScheduledRun, formatScheduledTaskFrequency, validateScheduledTaskFrequency } from "./scheduled-task-recurrence";

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

  // formatScheduledTaskFrequency returns i18next-ready { key, ...interpolation } data rather than
  // a pre-formatted string, so this module never needs to import i18next/React (see the exported
  // ScheduledTaskFrequencyLabel type). The caller is responsible for calling t(result.key, result)
  // and, for "weekly", for resolving the raw weekday index to a locale-native name first.
  it("returns a structured, i18next-ready label for interval frequencies", () => {
    expect(formatScheduledTaskFrequency({ kind: "minutes", interval: 1 })).toEqual({
      key: "scheduledTasks.frequency.summary.minutes",
      count: 1,
    });
    expect(formatScheduledTaskFrequency({ kind: "minutes", interval: 5 })).toEqual({
      key: "scheduledTasks.frequency.summary.minutes",
      count: 5,
    });
    expect(formatScheduledTaskFrequency({ kind: "hours", interval: 1 })).toEqual({
      key: "scheduledTasks.frequency.summary.hours",
      count: 1,
    });
    expect(formatScheduledTaskFrequency({ kind: "hours", interval: 3 })).toEqual({
      key: "scheduledTasks.frequency.summary.hours",
      count: 3,
    });
  });

  it("returns a structured, i18next-ready label for calendar frequencies", () => {
    expect(formatScheduledTaskFrequency({ kind: "daily", timeOfDay: "09:00" })).toEqual({
      key: "scheduledTasks.frequency.summary.daily",
      timeOfDay: "09:00",
    });
    expect(formatScheduledTaskFrequency({ kind: "weekly", weekday: 1, timeOfDay: "09:00" })).toEqual({
      key: "scheduledTasks.frequency.summary.weekly",
      weekday: 1,
      timeOfDay: "09:00",
    });
    expect(formatScheduledTaskFrequency({ kind: "monthly", dayOfMonth: 1, timeOfDay: "09:00" })).toEqual({
      key: "scheduledTasks.frequency.summary.monthly",
      dayOfMonth: 1,
      timeOfDay: "09:00",
    });
  });
});
