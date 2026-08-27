import { useEffect, useId, useRef, useState } from "react";
import { useTranslation } from "react-i18next";
import { cn } from "../lib/utils";
import type { WorkspaceContentMatch } from "../types/session-workspace-inspection";
import { useContentSearch } from "./use-content-search";

/**
 * Find a string inside the workspace, and go to where it is.
 *
 * A result is a position rather than a file, which is what makes this different from Quick Open:
 * selecting one opens the file *and* says which line, so the preview can put the reader on it
 * rather than at the top of a thousand-line file they then have to search again by eye.
 *
 * The same keyboard rules as Quick Open, for the same reason — arrows move, Enter opens, Escape
 * closes, focus stays in the input. A reader who has learned one surface should not have to learn
 * the other, and that includes the combobox wiring: without `aria-activedescendant` the DOM focus
 * never moves, so nothing announces which row Enter would take.
 */
export function ContentSearchPanel({
  isOpen,
  onClose,
  onSelect,
  sessionId,
}: {
  isOpen: boolean;
  onClose: () => void;
  onSelect: (match: WorkspaceContentMatch) => void;
  sessionId: string | null;
}) {
  const { t } = useTranslation();
  const inputRef = useRef<HTMLInputElement>(null);
  const [active, setActive] = useState(0);
  const search = useContentSearch(sessionId, isOpen);
  const listId = useId();
  const optionId = (index: number) => `${listId}-option-${index}`;

  useEffect(() => {
    if (isOpen) inputRef.current?.focus();
  }, [isOpen]);

  useEffect(() => {
    setActive(0);
  }, [search.matches]);

  if (!isOpen) return null;

  const choose = (match: WorkspaceContentMatch | undefined) => {
    if (!match) return;
    onSelect(match);
    onClose();
  };

  const onKeyDown = (event: React.KeyboardEvent<HTMLInputElement>) => {
    if (event.key === "Escape") {
      event.preventDefault();
      // Cancelled as well as closed. Closing alone would leave a full workspace scan running for a
      // reader who has already stopped looking at it.
      search.cancel();
      onClose();
      return;
    }
    if (event.key === "ArrowDown") {
      event.preventDefault();
      setActive((current) => Math.min(current + 1, Math.max(search.matches.length - 1, 0)));
      return;
    }
    if (event.key === "ArrowUp") {
      event.preventDefault();
      setActive((current) => Math.max(current - 1, 0));
      return;
    }
    if (event.key === "Enter") {
      event.preventDefault();
      choose(search.matches[active]);
    }
  };

  return (
    <div className="absolute inset-0 z-20 flex items-start justify-center bg-black/30 pt-16">
      <div
        aria-label={t("sessionTabs.files.contentSearch.title")}
        aria-modal="true"
        className="flex max-h-[70%] w-[min(680px,92%)] flex-col overflow-hidden rounded-lg border border-border bg-[hsl(var(--panel))] shadow-lg"
        role="dialog"
      >
        <input
          aria-activedescendant={search.matches.length > 0 ? optionId(active) : undefined}
          aria-autocomplete="list"
          aria-controls={listId}
          aria-expanded={search.matches.length > 0}
          aria-label={t("sessionTabs.files.contentSearch.placeholder")}
          className="w-full border-b border-border bg-transparent px-3 py-2 text-sm outline-none"
          onChange={(event) => search.setQuery(event.target.value)}
          onKeyDown={onKeyDown}
          placeholder={t("sessionTabs.files.contentSearch.placeholder")}
          ref={inputRef}
          role="combobox"
          type="text"
          value={search.query}
        />
        <ul
          aria-label={t("sessionTabs.files.contentSearch.results")}
          className="min-h-0 flex-1 overflow-y-auto p-1"
          id={listId}
          role="listbox"
        >
          {search.matches.map((match, index) => (
            <li
              aria-selected={index === active}
              className={cn(
                "flex w-full cursor-default flex-col items-start gap-0.5 rounded px-2 py-1.5 text-left hover:bg-muted",
                index === active && "bg-muted",
              )}
              id={optionId(index)}
              key={`${match.path}:${match.line}:${match.column}`}
              onClick={() => choose(match)}
              onMouseDown={(event) => event.preventDefault()}
              role="option"
            >
              <span className="truncate text-xs text-muted-foreground">
                {`${match.path}:${match.line}:${match.column}`}
              </span>
              <span className="w-full truncate font-mono text-xs">
                {match.snippet}
                {match.snippetTruncated ? "…" : null}
              </span>
            </li>
          ))}
        </ul>
        {search.matches.length === 0 && !search.isSearching ? (
          <p className="px-3 py-3 text-sm text-muted-foreground" role="status">
            {search.failed
              ? t("sessionTabs.files.contentSearch.failed")
              : search.query.trim()
                ? t("sessionTabs.files.contentSearch.empty")
                : t("sessionTabs.files.contentSearch.prompt")}
          </p>
        ) : null}
        {search.isSearching || (search.coverage && search.coverage.state !== "complete") ? (
          <div className="flex items-center justify-between gap-2 border-t border-border px-3 py-2 text-xs text-muted-foreground">
            <span>
              {search.isSearching
                ? t("sessionTabs.files.contentSearch.searching")
                : t(`sessionTabs.files.contentSearch.coverage.${search.coverage?.state}`)}
            </span>
            {search.isSearching ? (
              <button
                className="rounded border border-border px-2 py-1 hover:bg-muted"
                onClick={search.cancel}
                onMouseDown={(event) => event.preventDefault()}
                type="button"
              >
                {t("sessionTabs.files.contentSearch.cancel")}
              </button>
            ) : null}
          </div>
        ) : null}
      </div>
    </div>
  );
}
