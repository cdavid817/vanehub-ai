import type { ReactNode } from "react";

export interface DataTableColumn<T> {
  id: string;
  /** Already-localized column header. */
  header: string;
  cell: (row: T) => ReactNode;
  sortable?: boolean;
  align?: "start" | "end";
}

export interface DataTableSort {
  columnId: string;
  direction: "asc" | "desc";
}

export interface DataTablePage {
  index: number;
  size: number;
  totalCount: number;
}
