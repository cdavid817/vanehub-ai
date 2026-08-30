import { useRef, type ReactNode, type RefObject } from "react";
import { cn } from "../../lib/utils";
import { useSearchShortcut } from "./use-search-shortcut";

export interface ToolbarProps {
  /** The search input itself, caller-rendered — Toolbar only focuses it on the shortcut key. */
  search?: ReactNode;
  /** Ref to the actual focusable search control inside `search`; the shortcut is a no-op without it. */
  searchInputRef?: RefObject<HTMLElement | null>;
  onFilterTrigger?: () => void;
  filterTriggerLabel?: string;
  /** Rendered active-filter chips, typically from `FilterBar`. */
  activeFilters?: ReactNode;
  sortControl?: ReactNode;
  viewControl?: ReactNode;
  batchModeSlot?: ReactNode;
  className?: string;
}

export function Toolbar({
  search,
  searchInputRef,
  onFilterTrigger,
  filterTriggerLabel,
  activeFilters,
  sortControl,
  viewControl,
  batchModeSlot,
  className,
}: ToolbarProps) {
  const unusedRef = useRef<HTMLElement | null>(null);
  useSearchShortcut(searchInputRef ?? unusedRef);

  return (
    <div className={cn("flex flex-col gap-2", className)}>
      <div className="flex flex-wrap items-center gap-2">
        {search}
        {onFilterTrigger ? (
          <button
            className="ucd-focus-ring rounded-md border border-border px-2.5 py-1.5 text-sm hover:bg-accent"
            onClick={onFilterTrigger}
            type="button"
          >
            {filterTriggerLabel}
          </button>
        ) : null}
        <div className="ml-auto flex items-center gap-2">
          {sortControl}
          {viewControl}
        </div>
        {batchModeSlot}
      </div>
      {activeFilters}
    </div>
  );
}
