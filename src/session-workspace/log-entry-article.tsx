import { cn } from "../lib/utils";
import type { SessionLogEntry } from "../types/session-workspace";

export function LogEntryArticle({
  entry,
  focused,
  language,
  onFocused,
  position,
  total,
}: {
  entry: SessionLogEntry;
  focused: boolean;
  language: string;
  onFocused: () => void;
  position: number;
  total: number;
}) {
  return (
    <article
      aria-posinset={position}
      aria-setsize={total}
      className="rounded border border-border bg-background p-2 focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring"
      data-log-id={entry.id}
      ref={(element) => {
        if (!element || !focused) return;
        element.focus({ preventScroll: true });
        queueMicrotask(onFocused);
      }}
      role="listitem"
      tabIndex={-1}
    >
      <div className="flex items-center justify-between gap-2 text-xs">
        <span className={cn(
          "font-semibold uppercase",
          entry.level === "error" && "text-destructive",
          entry.level === "warn" && "text-primary",
        )}>
          {entry.level}
        </span>
        <time className="text-muted-foreground">
          {new Intl.DateTimeFormat(language, { dateStyle: "short", timeStyle: "medium" }).format(new Date(entry.timestamp))}
        </time>
      </div>
      <p className="mt-1 text-xs text-muted-foreground">{entry.category}</p>
      <p className="mt-1 whitespace-pre-wrap text-sm">{entry.message}</p>
      {Object.keys(entry.context).length > 0 ? (
        <pre className="mt-2 overflow-auto rounded bg-muted p-2 text-xs">{JSON.stringify(entry.context, null, 2)}</pre>
      ) : null}
    </article>
  );
}
