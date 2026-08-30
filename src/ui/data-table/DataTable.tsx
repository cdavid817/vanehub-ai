import { useRef } from "react";
import { DataTableBody, type DataTableBodyProps } from "./DataTableBody";
import { useTableCompactMode } from "./use-table-compact-mode";

export type DataTableProps<T> = Omit<DataTableBodyProps<T>, "compact">;

/** Measures its own container and falls back to a stacked-card layout below `COMPACT_MAX_WIDTH`. */
export function DataTable<T>(props: DataTableProps<T>) {
  const containerRef = useRef<HTMLDivElement>(null);
  const compact = useTableCompactMode(containerRef);

  return (
    <div ref={containerRef}>
      <DataTableBody {...props} compact={compact} />
    </div>
  );
}
