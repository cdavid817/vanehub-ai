import { memo } from "react";
import { cn } from "../../lib/utils";

/**
 * One selectable source line. A button rather than a div so the row is reachable and
 * activatable from the keyboard without reimplementing either.
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
        "flex w-full items-start gap-3 px-2 text-left leading-5 hover:bg-muted/60",
        selected && "bg-primary/15",
      )}
      data-selected={selected ? "true" : undefined}
      data-testid={`preview-line-${number}`}
      onClick={onSelect}
      type="button"
    >
      <span className="w-12 shrink-0 select-none pr-1 text-right tabular-nums text-muted-foreground">
        {number}
      </span>
      <span className="min-w-0 flex-1 whitespace-pre-wrap break-words" dangerouslySetInnerHTML={{ __html: html }} />
    </button>
  );
});
