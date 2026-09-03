import type { ScheduledTaskFrequency } from "../types/agent";

const timePattern = /^([01]\d|2[0-3]):([0-5]\d)$/;

function parseTimeOfDay(timeOfDay: string) {
  const match = timePattern.exec(timeOfDay);
  if (!match) throw new Error("Invalid time of day");
  return { hours: Number(match[1]), minutes: Number(match[2]) };
}

function startOfMinute(date: Date) {
  const value = new Date(date);
  value.setSeconds(0, 0);
  return value;
}

function daysInMonth(year: number, month: number) {
  return new Date(year, month + 1, 0).getDate();
}

function setTime(date: Date, timeOfDay: string) {
  const { hours, minutes } = parseTimeOfDay(timeOfDay);
  const value = new Date(date);
  value.setHours(hours, minutes, 0, 0);
  return value;
}

function nextDailyRun(from: Date, timeOfDay: string) {
  const candidate = setTime(from, timeOfDay);
  if (candidate > from) return candidate;
  candidate.setDate(candidate.getDate() + 1);
  return candidate;
}

function nextWeeklyRun(from: Date, weekday: number, timeOfDay: string) {
  if (!Number.isInteger(weekday) || weekday < 0 || weekday > 6) throw new Error("Invalid weekday");
  const candidate = setTime(from, timeOfDay);
  const dayDelta = (weekday - candidate.getDay() + 7) % 7;
  candidate.setDate(candidate.getDate() + dayDelta);
  if (candidate > from) return candidate;
  candidate.setDate(candidate.getDate() + 7);
  return candidate;
}

function monthlyCandidate(from: Date, monthOffset: number, dayOfMonth: number, timeOfDay: string) {
  if (!Number.isInteger(dayOfMonth) || dayOfMonth < 1 || dayOfMonth > 31) {
    throw new Error("Invalid day of month");
  }
  const year = from.getFullYear();
  const month = from.getMonth() + monthOffset;
  const candidate = setTime(new Date(year, month, 1), timeOfDay);
  candidate.setDate(Math.min(dayOfMonth, daysInMonth(candidate.getFullYear(), candidate.getMonth())));
  return candidate;
}

function nextMonthlyRun(from: Date, dayOfMonth: number, timeOfDay: string) {
  const candidate = monthlyCandidate(from, 0, dayOfMonth, timeOfDay);
  return candidate > from ? candidate : monthlyCandidate(from, 1, dayOfMonth, timeOfDay);
}

export function validateScheduledTaskFrequency(frequency: ScheduledTaskFrequency) {
  switch (frequency.kind) {
    case "minutes":
    case "hours":
      if (!Number.isInteger(frequency.interval) || frequency.interval <= 0) throw new Error("Invalid interval");
      return;
    case "daily":
      parseTimeOfDay(frequency.timeOfDay);
      return;
    case "weekly":
      nextWeeklyRun(new Date(), frequency.weekday, frequency.timeOfDay);
      return;
    case "monthly":
      nextMonthlyRun(new Date(), frequency.dayOfMonth, frequency.timeOfDay);
      return;
    default: {
      const exhaustive: never = frequency;
      throw new Error(`Unsupported frequency: ${String(exhaustive)}`);
    }
  }
}

/**
 * Field-by-field, not `JSON.stringify` equality -- key order in two independently-constructed
 * objects of the same shape is not guaranteed to match, and a false "different" here would make a
 * caller recompute `next_run_at` for an edit that never touched the schedule at all (task 19.8's
 * own Rust-side fix for the identical bug, `update_scheduled_task`'s doc comment).
 */
export function sameScheduledTaskFrequency(left: ScheduledTaskFrequency, right: ScheduledTaskFrequency): boolean {
  if (left.kind !== right.kind) return false;
  switch (left.kind) {
    case "minutes":
    case "hours":
      return left.interval === (right as typeof left).interval;
    case "daily":
      return left.timeOfDay === (right as typeof left).timeOfDay;
    case "weekly": {
      const other = right as typeof left;
      return left.weekday === other.weekday && left.timeOfDay === other.timeOfDay;
    }
    case "monthly": {
      const other = right as typeof left;
      return left.dayOfMonth === other.dayOfMonth && left.timeOfDay === other.timeOfDay;
    }
  }
}

export function computeNextScheduledRun(frequency: ScheduledTaskFrequency, from = new Date()) {
  validateScheduledTaskFrequency(frequency);
  const base = startOfMinute(from);
  switch (frequency.kind) {
    case "minutes": {
      const value = new Date(base);
      value.setMinutes(value.getMinutes() + frequency.interval);
      return value.toISOString();
    }
    case "hours": {
      const value = new Date(base);
      value.setHours(value.getHours() + frequency.interval);
      return value.toISOString();
    }
    case "daily":
      return nextDailyRun(from, frequency.timeOfDay).toISOString();
    case "weekly":
      return nextWeeklyRun(from, frequency.weekday, frequency.timeOfDay).toISOString();
    case "monthly":
      return nextMonthlyRun(from, frequency.dayOfMonth, frequency.timeOfDay).toISOString();
  }
}

/**
 * 19.12: the next N occurrences, built by feeding `computeNextScheduledRun`'s own result back in
 * as the next call's `from` -- not a second, parallel calculation. This is the only thing that
 * makes the preview provably share execution's exact semantics: the due-task sweep and this
 * preview both ultimately reduce to the same single-step function, so a future change to how one
 * occurrence is computed cannot silently drift the preview out of sync with what will actually
 * run. `count <= 0` returns an empty list rather than throwing -- there is no invalid input here,
 * just nothing to preview.
 */
export function computeNextScheduledOccurrences(
  frequency: ScheduledTaskFrequency,
  count: number,
  from = new Date(),
): string[] {
  const occurrences: string[] = [];
  let cursor = from;
  for (let index = 0; index < count; index += 1) {
    const next = computeNextScheduledRun(frequency, cursor);
    occurrences.push(next);
    cursor = new Date(next);
  }
  return occurrences;
}

export type ScheduledTaskFrequencyLabel =
  | { key: "scheduledTasks.frequency.summary.minutes"; count: number }
  | { key: "scheduledTasks.frequency.summary.hours"; count: number }
  | { key: "scheduledTasks.frequency.summary.daily"; timeOfDay: string }
  | { key: "scheduledTasks.frequency.summary.weekly"; weekday: number; timeOfDay: string }
  | { key: "scheduledTasks.frequency.summary.monthly"; dayOfMonth: number; timeOfDay: string };

/**
 * Returns i18next-ready `{ key, ...interpolation }` data instead of a pre-formatted string, so
 * this module can stay framework-agnostic (no i18next/React import here) while the caller does
 * `t(result.key, result)`. Weekday naming is deliberately left as the raw 0-6 domain index —
 * turning it into a locale-native name is the caller's job, e.g. via `formatAppWeekdayNames` in
 * `src/i18n/format.ts`.
 */
export function formatScheduledTaskFrequency(frequency: ScheduledTaskFrequency): ScheduledTaskFrequencyLabel {
  switch (frequency.kind) {
    case "minutes":
      return { key: "scheduledTasks.frequency.summary.minutes", count: frequency.interval };
    case "hours":
      return { key: "scheduledTasks.frequency.summary.hours", count: frequency.interval };
    case "daily":
      return { key: "scheduledTasks.frequency.summary.daily", timeOfDay: frequency.timeOfDay };
    case "weekly":
      return { key: "scheduledTasks.frequency.summary.weekly", weekday: frequency.weekday, timeOfDay: frequency.timeOfDay };
    case "monthly":
      return { key: "scheduledTasks.frequency.summary.monthly", dayOfMonth: frequency.dayOfMonth, timeOfDay: frequency.timeOfDay };
  }
}
