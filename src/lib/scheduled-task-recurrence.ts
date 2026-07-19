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

export function formatScheduledTaskFrequency(frequency: ScheduledTaskFrequency, weekdayLabels: string[]) {
  switch (frequency.kind) {
    case "minutes":
      return frequency.interval === 1 ? "Every minute" : `Every ${frequency.interval} minutes`;
    case "hours":
      return frequency.interval === 1 ? "Every hour" : `Every ${frequency.interval} hours`;
    case "daily":
      return `Daily at ${frequency.timeOfDay}`;
    case "weekly":
      return `Weekly ${weekdayLabels[frequency.weekday] ?? String(frequency.weekday)} at ${frequency.timeOfDay}`;
    case "monthly":
      return `Monthly on day ${frequency.dayOfMonth} at ${frequency.timeOfDay}`;
  }
}
