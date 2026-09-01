import { useEffect, useId, useMemo, useState, type KeyboardEvent } from "react";
import { Search } from "lucide-react";
import { useTranslation } from "react-i18next";
import { cn } from "../lib/utils";
import { searchSettingsIndex, type SettingsSearchEntry, type SettingsSearchResult } from "./settings-search-index";
import type { SettingsPageDefinition } from "./settings-page-types";

/**
 * Task 12.4-12.6: cross-page settings search, added alongside the existing per-page
 * `searchTerm`/`onSearchTermChange` wiring rather than replacing it (task 12.7 removes the old
 * page-local-only behavior in a later, separate step, once this has proven full-index parity).
 *
 * Combobox/listbox shape mirrors `command-center.tsx`'s own pattern deliberately, for one
 * consistent "type to search, arrow to navigate, Enter to choose" model across the app rather than
 * a second bespoke one -- differs only in staying an inline dropdown under the existing top-bar
 * input instead of a full-screen palette, since this search lives inside an already-open surface.
 */
export function SettingsSearchBox({
  index,
  onSearchTermChange,
  onSelectResult,
  pages,
  placeholder,
  searchTerm,
}: {
  index: SettingsSearchEntry[];
  onSearchTermChange: (value: string) => void;
  onSelectResult: (result: SettingsSearchResult) => void;
  pages: SettingsPageDefinition[];
  placeholder: string;
  searchTerm: string;
}) {
  const { t } = useTranslation();
  const [active, setActive] = useState(0);
  const listId = useId();
  const optionId = (index: number) => `${listId}-option-${index}`;

  const results = useMemo(
    () => searchSettingsIndex(index, pages, searchTerm, t),
    [index, pages, searchTerm, t],
  );
  const open = results.length > 0;

  useEffect(() => setActive(0), [searchTerm, results.length]);

  function choose(result: SettingsSearchResult | undefined) {
    if (!result) return;
    onSelectResult(result);
  }

  function onKeyDown(event: KeyboardEvent<HTMLInputElement>) {
    if (!open) return;
    if (event.key === "ArrowDown") {
      event.preventDefault();
      setActive((current) => Math.min(current + 1, results.length - 1));
      return;
    }
    if (event.key === "ArrowUp") {
      event.preventDefault();
      setActive((current) => Math.max(current - 1, 0));
      return;
    }
    if (event.key === "Enter") {
      event.preventDefault();
      choose(results[active]);
    }
  }

  return (
    <div className="relative min-w-0">
      <Search className="pointer-events-none absolute left-3 top-1/2 h-4 w-4 -translate-y-1/2 text-muted-foreground" />
      <input
        aria-activedescendant={open ? optionId(active) : undefined}
        aria-autocomplete="list"
        aria-controls={listId}
        aria-expanded={open}
        className="ucd-input h-9 w-full rounded-md px-9 text-sm outline-hidden focus-visible:ring-2 focus-visible:ring-ring"
        onChange={(event) => onSearchTermChange(event.target.value)}
        onKeyDown={onKeyDown}
        placeholder={placeholder}
        role="combobox"
        value={searchTerm}
      />
      {open ? (
        <ul
          aria-label={t("settings.search.resultsLabel")}
          className="absolute left-0 top-full z-20 mt-1 max-h-80 w-full min-w-[280px] overflow-y-auto rounded-md border border-border bg-background py-1 shadow-lg"
          id={listId}
          role="listbox"
        >
          {results.map((result, resultIndex) => (
            <li
              aria-selected={resultIndex === active}
              className={cn(
                "flex cursor-default flex-col gap-0.5 px-3 py-2 text-left text-sm hover:bg-muted",
                resultIndex === active && "bg-muted text-primary",
              )}
              id={optionId(resultIndex)}
              key={`${result.page.id}:${result.entry.kind}:${result.entry.labelKey}`}
              onClick={() => choose(result)}
              onMouseDown={(event) => event.preventDefault()}
              role="option"
            >
              <span className="truncate font-medium">{t(result.entry.labelKey)}</span>
              <span className="truncate text-xs text-muted-foreground">
                {result.entry.kind === "field" ? t(result.page.labelKey) : t(result.page.descriptionKey)}
              </span>
            </li>
          ))}
        </ul>
      ) : null}
      {searchTerm.trim() && !open ? (
        <div className="absolute left-0 top-full z-20 mt-1 w-full min-w-[280px] rounded-md border border-border bg-background px-3 py-3 text-sm text-muted-foreground shadow-lg" role="status">
          {t("settings.search.noResults")}
        </div>
      ) : null}
    </div>
  );
}
