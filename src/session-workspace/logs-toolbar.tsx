import { ArrowUpToLine, Clock3, Download, Pause, Play, Search } from "lucide-react";
import { useTranslation } from "react-i18next";
import { cn } from "../lib/utils";
import type { SessionLogLevel } from "../types/session-workspace";

export const LOG_LEVELS: SessionLogLevel[] = ["error", "warn", "info", "debug"];

export interface LogsToolbarProps {
  levels: SessionLogLevel[];
  onToggleLevel: (level: SessionLogLevel) => void;
  searchDraft: string;
  onSearchDraftChange: (value: string) => void;
  onSubmitSearch: () => void;
  timestampDraft: string;
  onTimestampDraftChange: (value: string) => void;
  onLocate: () => void;
  seeking: boolean;
  onExport: () => void;
  /** Whether the viewport is currently allowed to move to new rows by itself. */
  following: boolean;
  paused: boolean;
  onTogglePause: () => void;
  /** Rows that arrived while the view was held still. Zero while following. */
  pendingCount: number;
  onJumpToLatest: () => void;
}

/**
 * Everything above the list.
 *
 * Split out because the tab was already at the line rule before Follow, Jump to latest and the
 * scope chips existed, and because these controls have no state of their own — they are a
 * projection of the tab's, which is what keeps "what the toolbar shows" and "what the query asks
 * for" from drifting into two answers.
 */
export function LogsToolbar({
  following,
  levels,
  onExport,
  onJumpToLatest,
  onLocate,
  onSearchDraftChange,
  onSubmitSearch,
  onTimestampDraftChange,
  onToggleLevel,
  onTogglePause,
  paused,
  pendingCount,
  searchDraft,
  seeking,
  timestampDraft,
}: LogsToolbarProps) {
  const { t } = useTranslation();

  return (
    <div className="flex flex-wrap items-center gap-2 rounded-lg border border-border bg-[hsl(var(--panel-muted))] p-2">
      {LOG_LEVELS.map((level) => (
        <button
          aria-pressed={levels.includes(level)}
          className={cn(
            "h-7 rounded border border-border px-2 text-xs uppercase",
            levels.includes(level)
              ? "bg-primary text-primary-foreground"
              : "bg-background text-muted-foreground",
          )}
          key={level}
          onClick={() => onToggleLevel(level)}
          type="button"
        >
          {t(`sessionTabs.logs.level.${level}`)}
        </button>
      ))}
      <button
        // `aria-pressed` states the choice rather than its effect. A reader who scrolled away is
        // also not following, and a control that lit up for that would be reporting something the
        // reader never pressed.
        aria-pressed={paused}
        className={cn(
          "flex h-8 items-center gap-1 rounded border border-border px-2 text-xs",
          paused ? "bg-primary text-primary-foreground" : "bg-background hover:bg-muted",
        )}
        onClick={onTogglePause}
        type="button"
      >
        {paused ? <Play className="h-3.5 w-3.5" aria-hidden="true" /> : <Pause className="h-3.5 w-3.5" aria-hidden="true" />}
        {paused ? t("sessionTabs.logs.follow") : t("sessionTabs.logs.pause")}
      </button>
      {following ? null : (
        // Shown only while the view is held still, because that is the only time it says anything.
        // Following, it would be a button that scrolls to where the reader already is.
        <button
          className="flex h-8 items-center gap-1 rounded border border-border bg-background px-2 text-xs hover:bg-muted"
          onClick={onJumpToLatest}
          type="button"
        >
          <ArrowUpToLine className="h-3.5 w-3.5" aria-hidden="true" />
          {pendingCount > 0
            ? t("sessionTabs.logs.jumpToLatestCount", { count: pendingCount })
            : t("sessionTabs.logs.jumpToLatest")}
        </button>
      )}
      <form
        className="ml-auto flex min-w-48 flex-1 items-center gap-1 sm:max-w-sm"
        onSubmit={(event) => {
          event.preventDefault();
          onSubmitSearch();
        }}
      >
        <input
          aria-label={t("sessionTabs.logs.search")}
          className="ucd-input h-8 min-w-0 flex-1 rounded px-2 text-sm"
          onChange={(event) => onSearchDraftChange(event.target.value)}
          placeholder={t("sessionTabs.logs.search")}
          value={searchDraft}
        />
        <button
          className="flex h-8 w-8 items-center justify-center rounded border border-border hover:bg-muted"
          title={t("sessionTabs.logs.search")}
          type="submit"
        >
          <Search className="h-4 w-4" aria-hidden="true" />
        </button>
      </form>
      <form
        className="flex min-w-64 flex-1 items-center gap-1 sm:max-w-md"
        onSubmit={(event) => {
          event.preventDefault();
          onLocate();
        }}
      >
        <input
          aria-label={t("sessionTabs.logs.timestamp")}
          className="ucd-input h-8 min-w-0 flex-1 rounded px-2 text-sm"
          onChange={(event) => onTimestampDraftChange(event.target.value)}
          type="datetime-local"
          value={timestampDraft}
        />
        <button
          className="flex h-8 items-center gap-1 rounded border border-border px-2 text-xs hover:bg-muted"
          disabled={seeking}
          type="submit"
        >
          <Clock3 className="h-3.5 w-3.5" aria-hidden="true" />
          {seeking ? t("sessionTabs.logs.seeking") : t("sessionTabs.logs.locate")}
        </button>
      </form>
      <button
        className="flex h-8 items-center gap-1 rounded border border-border px-2 text-xs hover:bg-muted"
        onClick={onExport}
        type="button"
      >
        <Download className="h-3.5 w-3.5" aria-hidden="true" />
        {t("sessionTabs.logs.export")}
      </button>
    </div>
  );
}
