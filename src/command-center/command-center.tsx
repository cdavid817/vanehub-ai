import { useEffect, useId, useMemo, useState, type KeyboardEvent } from "react";
import { useTranslation } from "react-i18next";
import { cn } from "../lib/utils";
import { useFocusTrap } from "../ui/sheet/use-focus-trap";
import { COMMANDS, SEARCH_PROVIDERS } from "./command-center-registry";
import { commandCenterShortcutLabel } from "./platform";
import { useCommandCenterSearch } from "./use-command-center-search";
import type { WorkbenchCommand, WorkbenchCommandContext, WorkbenchSearchProvider, WorkbenchSearchResult } from "./command-center-types";

type PaletteEntry =
  | { kind: "result"; entryKey: string; result: WorkbenchSearchResult }
  | { kind: "command"; entryKey: string; command: WorkbenchCommand };

/**
 * 6.2/6.9. Uses `useFocusTrap` directly rather than `ApplicationDialog`: that component's chrome
 * is a title/description/children/footer shape for form-like dialogs, not the single
 * input-over-listbox shape a command palette needs — `QuickOpenDialog`
 * (session-workspace/quick-open-dialog.tsx) is the closer precedent for the combobox/listbox
 * keyboard-nav pattern itself, just without its own focus-trap (that dialog is scoped inside the
 * session tab area; this one opens globally and must be aware of an already-open modal above it).
 *
 * 6.9's "hidden... with an accessible explanation" is satisfied by filtering `COMMANDS` through
 * `isAvailable(context)` before they ever reach the list — see `contextual-commands.ts`'s own doc
 * comment for why hiding, not disabling, is the right shape for a destination-scoped command.
 */
export function CommandCenter({
  context,
  onClose,
  providers = SEARCH_PROVIDERS,
}: {
  context: WorkbenchCommandContext;
  onClose: () => void;
  /** Defaults to the real registry; a test-only seam, same rationale as `useCommandCenterSearch`'s
   *  own `providers` parameter (see that hook's doc comment). */
  providers?: WorkbenchSearchProvider[];
}) {
  const { t } = useTranslation();
  const [query, setQuery] = useState("");
  const [active, setActive] = useState(0);
  const dialogRef = useFocusTrap<HTMLElement>({ onClose });
  const listId = useId();
  const optionId = (index: number) => `${listId}-option-${index}`;

  const search = useCommandCenterSearch(query, providers);

  const matchedCommands = useMemo(() => {
    const normalized = query.trim().toLowerCase();
    const available = COMMANDS.filter((command) => command.isAvailable(context));
    if (!normalized) return available;
    return available.filter((command) => t(command.labelKey).toLowerCase().includes(normalized)
      || command.keywords.some((keyword) => keyword.toLowerCase().includes(normalized)));
  }, [query, context, t]);

  const entries: PaletteEntry[] = [
    ...search.results.map((result): PaletteEntry => ({ kind: "result", entryKey: `result:${result.key}`, result })),
    ...matchedCommands.map((command): PaletteEntry => ({ kind: "command", entryKey: `command:${command.id}`, command })),
  ];

  // Back to the top whenever the query OR the async result set changes — commands update
  // synchronously with the query, but `search.results` lands later, after the debounce settles.
  // Resetting on `query` alone would leave the highlight on whatever row a result's insertion
  // happens to shift into that position, a different entry than the one the reader was looking at
  // (same rationale as `QuickOpenDialog`'s own reset effect, extended to this hook's async gap).
  useEffect(() => setActive(0), [query, search.results]);

  const choose = (entry: PaletteEntry | undefined) => {
    if (!entry) return;
    if (entry.kind === "result") context.navigate(entry.result.route);
    else void entry.command.run(context);
    onClose();
  };

  const onKeyDown = (event: KeyboardEvent<HTMLInputElement>) => {
    if (event.key === "ArrowDown") {
      event.preventDefault();
      setActive((current) => Math.min(current + 1, Math.max(entries.length - 1, 0)));
      return;
    }
    if (event.key === "ArrowUp") {
      event.preventDefault();
      setActive((current) => Math.max(current - 1, 0));
      return;
    }
    if (event.key === "Enter") {
      event.preventDefault();
      choose(entries[active]);
    }
    // Escape is not handled here: `useFocusTrap` already closes on Escape for the topmost modal,
    // and duplicating it here would race the same keystroke against two close paths.
  };

  return (
    <div
      className="fixed inset-0 z-50 flex items-start justify-center bg-black/50 p-3 pt-24 sm:p-5 sm:pt-24"
      onMouseDown={(event) => { if (event.target === event.currentTarget) onClose(); }}
      role="presentation"
    >
      <section
        aria-label={t("commandCenter.title")}
        aria-modal="true"
        className="flex max-h-[70vh] w-full max-w-xl flex-col overflow-hidden rounded-xl border border-border bg-background shadow-2xl"
        ref={dialogRef}
        role="dialog"
        tabIndex={-1}
      >
        <div className="flex items-center gap-2 border-b border-border px-4 py-3">
          <input
            aria-activedescendant={entries.length > 0 ? optionId(active) : undefined}
            aria-autocomplete="list"
            aria-controls={listId}
            aria-expanded={entries.length > 0}
            className="w-full bg-transparent text-sm outline-none"
            data-dialog-autofocus
            onChange={(event) => setQuery(event.target.value)}
            onKeyDown={onKeyDown}
            placeholder={t("commandCenter.placeholder")}
            role="combobox"
            type="text"
            value={query}
          />
          <kbd className="shrink-0 rounded border border-border px-1.5 py-0.5 text-xs text-muted-foreground">
            {t("commandCenter.shortcutHint", { shortcut: commandCenterShortcutLabel() })}
          </kbd>
        </div>
        <ul aria-label={t("commandCenter.resultsLabel")} className="min-h-0 flex-1 overflow-y-auto p-1" id={listId} role="listbox">
          {entries.map((entry, index) => (
            <li
              aria-selected={index === active}
              className={cn(
                "flex h-9 w-full cursor-default items-center gap-2 rounded px-2 text-left text-sm hover:bg-muted",
                index === active && "bg-muted text-primary",
              )}
              id={optionId(index)}
              key={entry.entryKey}
              onClick={() => choose(entry)}
              // Pointer focus would take it from the input and end the reader's ability to keep
              // typing, which is the entire point of the surface (same as `QuickOpenDialog`).
              onMouseDown={(event) => event.preventDefault()}
              role="option"
            >
              {entry.kind === "result" ? (
                <>
                  <span className="truncate">{entry.result.title}</span>
                  {entry.result.subtitle ? (
                    <span className="truncate text-xs text-muted-foreground">{entry.result.subtitle}</span>
                  ) : null}
                </>
              ) : (
                <span className="truncate">{t(entry.command.labelKey)}</span>
              )}
            </li>
          ))}
        </ul>
        {entries.length === 0 && !search.loading ? (
          <p className="px-4 py-3 text-sm text-muted-foreground" role="status">{t("commandCenter.empty")}</p>
        ) : null}
        {search.loading ? (
          <p className="px-4 py-2 text-xs text-muted-foreground" role="status">{t("commandCenter.loading")}</p>
        ) : null}
        {search.failedProviderIds.length > 0 ? (
          <p className="px-4 py-2 text-xs text-muted-foreground" role="status">{t("commandCenter.partialFailure")}</p>
        ) : null}
      </section>
    </div>
  );
}
