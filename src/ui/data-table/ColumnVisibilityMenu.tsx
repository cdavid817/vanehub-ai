import { useEffect, useRef, useState } from "react";
import { Columns3 } from "lucide-react";
import { useTranslation } from "react-i18next";
import { cn } from "../../lib/utils";

export interface ColumnVisibilityMenuProps {
  columns: { id: string; header: string }[];
  visibleColumnIds: readonly string[];
  onVisibleColumnIdsChange: (ids: string[]) => void;
  className?: string;
}

/**
 * Deliberately not built on `ActionMenu`: toggling a column should not close the popover the way
 * activating a menu item does, since a reader typically wants to flip several columns in one
 * interaction. Native checkboxes keep this reachable by Tab alone, so it does not need
 * `ActionMenu`'s custom roving-tabindex either.
 */
export function ColumnVisibilityMenu({ columns, visibleColumnIds, onVisibleColumnIdsChange, className }: ColumnVisibilityMenuProps) {
  const { t } = useTranslation();
  const [open, setOpen] = useState(false);
  const containerRef = useRef<HTMLDivElement>(null);
  const triggerRef = useRef<HTMLButtonElement>(null);
  const onlyOneVisible = visibleColumnIds.length === 1;

  function close() {
    setOpen(false);
    triggerRef.current?.focus();
  }

  useEffect(() => {
    if (!open) return;
    function handlePointerDown(event: PointerEvent) {
      if (!containerRef.current?.contains(event.target as Node)) setOpen(false);
    }
    document.addEventListener("pointerdown", handlePointerDown);
    return () => document.removeEventListener("pointerdown", handlePointerDown);
  }, [open]);

  function toggle(id: string) {
    const isVisible = visibleColumnIds.includes(id);
    if (isVisible && onlyOneVisible) return;
    onVisibleColumnIdsChange(isVisible ? visibleColumnIds.filter((value) => value !== id) : [...visibleColumnIds, id]);
  }

  return (
    <div className={cn("relative inline-block", className)} ref={containerRef}>
      <button
        aria-expanded={open}
        aria-haspopup="true"
        className="ucd-focus-ring inline-flex items-center gap-1.5 rounded-md border border-border px-2.5 py-1.5 text-sm hover:bg-accent"
        onClick={() => setOpen((value) => !value)}
        ref={triggerRef}
        type="button"
      >
        <Columns3 aria-hidden="true" className="h-3.5 w-3.5" />
        {t("workbenchUi.dataTable.columns")}
      </button>
      {open ? (
        <div
          className="ucd-raised absolute right-0 z-40 mt-1 min-w-40 rounded-md border border-border p-1 shadow-lg"
          onKeyDown={(event) => { if (event.key === "Escape") close(); }}
        >
          {columns.map((column) => {
            const checked = visibleColumnIds.includes(column.id);
            return (
              <label className="flex items-center gap-2 rounded-sm px-2 py-1.5 text-sm hover:bg-accent" key={column.id}>
                <input checked={checked} disabled={checked && onlyOneVisible} onChange={() => toggle(column.id)} type="checkbox" />
                {column.header}
              </label>
            );
          })}
          {onlyOneVisible ? (
            <p className="px-2 pb-1 pt-2 text-xs text-muted-foreground">{t("workbenchUi.dataTable.atLeastOneColumn")}</p>
          ) : null}
        </div>
      ) : null}
    </div>
  );
}
