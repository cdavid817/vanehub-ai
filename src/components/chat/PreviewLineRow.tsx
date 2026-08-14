import { memo } from "react";
import { cn } from "../../lib/utils";

/**
 * One selectable source line. A button rather than a div so the row is reachable and
 * activatable from the keyboard without reimplementing either.
 *
 * The line does not wrap: wrapping breaks code indentation and makes row heights vary,
 * which the virtualizer then has to re-measure mid-scroll. The list scrolls sideways
 * instead, and the line number stays pinned so it remains readable while it does.
 *
 * `html` is highlight.js output: it escapes the text it wraps, and the only markup it
 * emits is `<span class="hljs-*">`, so injecting it does not put file content into the
 * DOM as active markup.
 */
export const PreviewLineRow = memo(function PreviewLineRow({
  html,
  number,
  onSelect,
  selected,
}: {
  html: string;
  number: number;
  onSelect: () => void;
  selected: boolean;
}) {
  return (
    <button
      className={cn(
        "flex w-max min-w-full items-start gap-3 bg-[hsl(var(--panel-muted))] px-2 text-left leading-5",
        "focus-visible:relative focus-visible:z-20 focus-visible:outline-hidden focus-visible:ring-2 focus-visible:ring-ring",
        // Not `hover:` alongside the selected background: the two would fight, leaving a
        // selected row looking unselected under the pointer.
        selected ? "bg-primary/20" : "hover:bg-muted/60",
      )}
      data-selected={selected ? "true" : undefined}
      data-testid={`preview-line-${number}`}
      onClick={onSelect}
      type="button"
    >
      <span className="sticky left-0 z-10 w-12 shrink-0 select-none bg-inherit pr-2 text-right tabular-nums text-muted-foreground">
        {number}
      </span>
      <span className="whitespace-pre" dangerouslySetInnerHTML={{ __html: html }} />
    </button>
  );
});
