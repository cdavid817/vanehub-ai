import { useEffect, useId, useRef, useState } from "react";
import { File, Folder } from "lucide-react";
import { useTranslation } from "react-i18next";
import { cn } from "../lib/utils";
import type { WorkspacePathMatch } from "../types/session-workspace-inspection";
import { useQuickOpen } from "./use-quick-open";

/**
 * Reach a path by typing part of it.
 *
 * Keyboard-first because that is the only way it is faster than the tree. Arrow keys move, Enter
 * opens, Escape closes, and the input keeps focus throughout — a result list that stole focus on
 * arrow-down would end the reader's ability to keep typing, which is the whole interaction.
 *
 * The active row is tracked by index rather than by path. Results are replaced wholesale on each
 * keystroke, and a path-based selection would either vanish or, worse, silently point at a row that
 * is no longer where it was.
 *
 * Wired as a combobox over a listbox, which is the only shape that makes "focus stays in the input
 * while the highlight moves" announceable. Without `aria-activedescendant` the DOM focus never
 * moves, so a screen reader has nothing to read out when the reader presses the down arrow: they
 * hear the query they typed and nothing about the row Enter would take.
 */
export function QuickOpenDialog({
  isOpen,
  onClose,
  onSelect,
  sessionId,
}: {
  isOpen: boolean;
  onClose: () => void;
  onSelect: (match: WorkspacePathMatch) => void;
  sessionId: string | null;
}) {
  const { t } = useTranslation();
  const inputRef = useRef<HTMLInputElement>(null);
  const [active, setActive] = useState(0);
  const quickOpen = useQuickOpen(sessionId, isOpen);
  // Stable per mount, so the input's `aria-activedescendant` names an element that exists rather
  // than one that happened to be at that index when the string was built.
  const listId = useId();
  const optionId = (index: number) => `${listId}-option-${index}`;

  useEffect(() => {
    if (isOpen) inputRef.current?.focus();
  }, [isOpen]);

  // Back to the top whenever the result set changes. Keeping the index would leave the highlight on
  // whatever row happened to land at that position, which is a different file than the one the
  // reader was looking at.
  useEffect(() => {
    setActive(0);
  }, [quickOpen.matches]);

  if (!isOpen) return null;

  const choose = (match: WorkspacePathMatch | undefined) => {
    if (!match) return;
    onSelect(match);
    onClose();
  };

  const onKeyDown = (event: React.KeyboardEvent<HTMLInputElement>) => {
    if (event.key === "Escape") {
      event.preventDefault();
      onClose();
      return;
    }
    if (event.key === "ArrowDown") {
      event.preventDefault();
      setActive((current) => Math.min(current + 1, Math.max(quickOpen.matches.length - 1, 0)));
      return;
    }
    if (event.key === "ArrowUp") {
      event.preventDefault();
      setActive((current) => Math.max(current - 1, 0));
      return;
    }
    if (event.key === "Enter") {
      event.preventDefault();
      choose(quickOpen.matches[active]);
    }
  };

  return (
    <div className="absolute inset-0 z-20 flex items-start justify-center bg-black/30 pt-16">
      <div
        aria-label={t("sessionTabs.files.quickOpen.title")}
        aria-modal="true"
        className="flex max-h-[60%] w-[min(560px,90%)] flex-col overflow-hidden rounded-lg border border-border bg-[hsl(var(--panel))] shadow-lg"
        role="dialog"
      >
        <input
          aria-activedescendant={quickOpen.matches.length > 0 ? optionId(active) : undefined}
          aria-autocomplete="list"
          aria-controls={listId}
          aria-expanded={quickOpen.matches.length > 0}
          aria-label={t("sessionTabs.files.quickOpen.placeholder")}
          className="w-full border-b border-border bg-transparent px-3 py-2 text-sm outline-none"
          onChange={(event) => quickOpen.setQuery(event.target.value)}
          onKeyDown={onKeyDown}
          placeholder={t("sessionTabs.files.quickOpen.placeholder")}
          ref={inputRef}
          role="combobox"
          type="text"
          value={quickOpen.query}
        />
        <ul
          aria-label={t("sessionTabs.files.quickOpen.results")}
          className="min-h-0 flex-1 overflow-y-auto p-1"
          id={listId}
          role="listbox"
        >
          {quickOpen.matches.map((match, index) => (
            // The option is the list item itself. A button carrying `role="option"` inside an
            // `li` puts a listitem between the listbox and its options, which is not a shape the
            // accessibility tree allows — and the interactive descendant is what breaks the
            // activedescendant pairing above.
            <li
              aria-selected={index === active}
              className={cn(
                "flex h-8 w-full cursor-default items-center gap-2 rounded px-2 text-left text-sm hover:bg-muted",
                index === active && "bg-muted text-primary",
              )}
              id={optionId(index)}
              key={`${match.kind}:${match.path}`}
              onClick={() => choose(match)}
              // Pointer focus would take it from the input and end the reader's ability to keep
              // typing, which is the entire point of the surface.
              onMouseDown={(event) => event.preventDefault()}
              role="option"
            >
              {match.kind === "directory" ? (
                <Folder className="h-4 w-4 shrink-0 text-primary" />
              ) : (
                <File className="h-4 w-4 shrink-0 text-muted-foreground" />
              )}
              <span className="truncate">{match.path}</span>
            </li>
          ))}
        </ul>
        {/* Outside the listbox. A listbox may hold options and groups; a plain list item announces
            itself as one more thing to choose from, and there is nothing here to choose. */}
        {quickOpen.matches.length === 0 && !quickOpen.isLoading ? (
          <p className="px-3 py-3 text-sm text-muted-foreground" role="status">
            {quickOpen.failed
              ? t("sessionTabs.files.quickOpen.failed")
              : t("sessionTabs.files.quickOpen.empty")}
          </p>
        ) : null}
        <QuickOpenFooter
          coverageReason={
            quickOpen.coverage && quickOpen.coverage.state !== "complete"
              ? quickOpen.coverage.state
              : null
          }
          hasMore={Boolean(quickOpen.nextCursor)}
          isLoading={quickOpen.isLoading}
          onLoadMore={quickOpen.loadMore}
        />
      </div>
    </div>
  );
}

/**
 * What the list does not say on its own.
 *
 * "More matches follow" and "part of the workspace was never examined" are separate lines because
 * they are separate facts. A reader who pages to the end has resolved the first and not the second,
 * and one combined message would let them believe otherwise.
 */
function QuickOpenFooter({
  coverageReason,
  hasMore,
  isLoading,
  onLoadMore,
}: {
  coverageReason: string | null;
  hasMore: boolean;
  isLoading: boolean;
  onLoadMore: () => void;
}) {
  const { t } = useTranslation();
  if (!coverageReason && !hasMore && !isLoading) return null;
  return (
    <div className="flex items-center justify-between gap-2 border-t border-border px-3 py-2 text-xs text-muted-foreground">
      <span>
        {coverageReason ? t(`sessionTabs.files.quickOpen.coverage.${coverageReason}`) : null}
      </span>
      {hasMore ? (
        <button
          className="rounded border border-border px-2 py-1 hover:bg-muted"
          disabled={isLoading}
          onClick={onLoadMore}
          onMouseDown={(event) => event.preventDefault()}
          type="button"
        >
          {t("sessionTabs.files.quickOpen.more")}
        </button>
      ) : null}
    </div>
  );
}
