import { useEffect, useRef, useState } from "react";
import { ListFilter, X } from "lucide-react";
import { useTranslation } from "react-i18next";

export interface FilterFieldOption<V extends string> {
  value: V;
  label: string;
}

export interface FilterField<V extends string = string> {
  id: string;
  label: string;
  options: FilterFieldOption<V>[];
  value: V;
  onChange: (value: V) => void;
  /** The value meaning "no filter applied" for this field — a field at its default value is
   *  excluded from the active-filter chip row and does not count toward the active-count badge. */
  defaultValue: V;
}

export interface FilterPopoverProps {
  fields: FilterField[];
  /** Trigger button's accessible label/tooltip, e.g. "Filters". Translated by the caller. */
  triggerLabel: string;
}

/**
 * Generic trigger-button-plus-panel filter primitive: one native `<select>` per field, plus a
 * dismissible chip for every field away from its default. Modeled on `ColumnVisibilityMenu`'s own
 * pointerdown/Escape close rather than the shared `useFocusTrap` — this panel is not a modal, so it
 * must not join the trap's Tab-cycling or its "topmost `[aria-modal=true]`" stack check, both of
 * which assume a dialog that owns the page until dismissed.
 */
export function FilterPopover({ fields, triggerLabel }: FilterPopoverProps) {
  const { t } = useTranslation();
  const [open, setOpen] = useState(false);
  const containerRef = useRef<HTMLDivElement>(null);
  const triggerRef = useRef<HTMLButtonElement>(null);
  const activeFields = fields.filter((field) => field.value !== field.defaultValue);

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

  return (
    <div className="inline-flex flex-wrap items-center gap-1.5">
      <div className="relative inline-block" ref={containerRef}>
        <button
          aria-expanded={open}
          aria-haspopup="true"
          className="ucd-focus-ring inline-flex items-center gap-1.5 rounded-md border border-border px-2.5 py-1.5 text-sm hover:bg-accent"
          onClick={() => setOpen((value) => !value)}
          ref={triggerRef}
          type="button"
        >
          <ListFilter aria-hidden="true" className="h-3.5 w-3.5" />
          {triggerLabel}
          {activeFields.length > 0 ? (
            <span
              aria-hidden="true"
              className="inline-flex h-4 min-w-4 items-center justify-center rounded-full bg-primary px-1 text-[10px] font-semibold leading-none text-primary-foreground"
              data-testid="filter-popover-count"
            >
              {activeFields.length}
            </span>
          ) : null}
          {activeFields.length > 0 ? (
            <span className="sr-only">{t("workbenchUi.filterPopover.activeCount", { count: activeFields.length })}</span>
          ) : null}
        </button>
        {open ? (
          <div
            className="ucd-raised absolute right-0 z-40 mt-1 min-w-48 rounded-md border border-border p-2 shadow-lg"
            onKeyDown={(event) => { if (event.key === "Escape") close(); }}
          >
            {fields.map((field) => (
              <div className="grid gap-1 py-1 text-xs" key={field.id}>
                <span className="text-muted-foreground">{field.label}</span>
                <select
                  aria-label={field.label}
                  className="ucd-input h-8 rounded-md px-2 text-xs"
                  onChange={(event) => field.onChange(event.target.value)}
                  value={field.value}
                >
                  {field.options.map((option) => (
                    <option key={option.value} value={option.value}>{option.label}</option>
                  ))}
                </select>
              </div>
            ))}
          </div>
        ) : null}
      </div>
      {activeFields.map((field) => {
        const optionLabel = field.options.find((option) => option.value === field.value)?.label ?? field.value;
        return (
          <span className="ucd-status-neutral inline-flex items-center gap-1 rounded-sm border px-2 py-0.5 text-xs" key={field.id}>
            {field.label}: {optionLabel}
            <button
              aria-label={t("workbenchUi.filterPopover.clearFilter", { label: field.label })}
              className="ucd-focus-ring rounded-sm"
              onClick={() => field.onChange(field.defaultValue)}
              type="button"
            >
              <X aria-hidden="true" className="h-3 w-3" />
            </button>
          </span>
        );
      })}
    </div>
  );
}
