import { useTranslation } from "react-i18next";

/**
 * 19.13 + 19.15: two honest, display-only facts about how a scheduled task actually executes,
 * shared verbatim by the editor Review (`scheduled-task-review.tsx`) and the route-backed detail
 * view's own capability notice (`scheduled-task-capability-notice.tsx`) -- one component so the
 * two surfaces cannot drift into disagreeing about the same real behavior.
 *
 * 19.13's own literal task wording asks for "explicit configured timezone and daylight-saving
 * policy." Neither exists: scheduling is OS/process-local (`chrono::Local::now()` at every
 * recompute site in `scheduled_tasks.rs`), there is no per-task timezone field on either the Rust
 * (`dto::ScheduledTask`) or TS (`types/agent.ts`) side, and DST handling is exactly one silent
 * crash-avoidance fallback (`next_daily`'s `.single().unwrap_or(from)`), not a stated policy. Per
 * this change's own established pattern for a requested surface that does not exist server-side
 * (16.5's "Attention" filter, correctly not built rather than faked), this shows the real fact --
 * this device's own current IANA zone, read live via `Intl` -- instead of a `<select>` implying a
 * stored per-task choice that does not exist.
 *
 * 19.15 reuses `scheduledTasks.runtimeHint` verbatim rather than a second, slightly-different
 * paraphrase -- it already states the application-open + at-most-one-catch-up model precisely
 * (`ScheduledTaskForm` already shows it during editing); repeating the same key here means the
 * three surfaces (form, Review, detail) can never quietly say three different things about the
 * same real behavior.
 */
export function ScheduledTaskExecutionNotice() {
  const { t } = useTranslation();
  const zone = Intl.DateTimeFormat().resolvedOptions().timeZone;
  return (
    <div className="grid gap-1.5 text-xs text-muted-foreground" data-testid="scheduled-task-execution-notice">
      <p>{t("scheduledTasks.executionNotice.timezone", { zone })}</p>
      <p>{t("scheduledTasks.runtimeHint")}</p>
    </div>
  );
}
