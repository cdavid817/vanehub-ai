import type { ReactNode } from "react";
import { ArrowDown, ArrowUp, ArrowUpDown } from "lucide-react";
import { useTranslation } from "react-i18next";
import { cn } from "../../lib/utils";
import { ColumnVisibilityMenu } from "./ColumnVisibilityMenu";
import type { DataTableColumn, DataTablePage, DataTableSort } from "./types";

export interface DataTableBodyProps<T> {
  compact: boolean;
  columns: DataTableColumn<T>[];
  rows: readonly T[];
  rowKey: (row: T) => string;
  ariaLabel: string;
  sort?: DataTableSort;
  onSortChange?: (sort: DataTableSort) => void;
  visibleColumnIds?: readonly string[];
  onVisibleColumnIdsChange?: (ids: string[]) => void;
  selectedRowKeys?: ReadonlySet<string>;
  onSelectedRowKeysChange?: (keys: ReadonlySet<string>) => void;
  page?: DataTablePage;
  onPageChange?: (index: number) => void;
  emptyState?: ReactNode;
  className?: string;
}

export function DataTableBody<T>({
  compact,
  columns,
  rows,
  rowKey,
  ariaLabel,
  sort,
  onSortChange,
  visibleColumnIds,
  onVisibleColumnIdsChange,
  selectedRowKeys,
  onSelectedRowKeysChange,
  page,
  onPageChange,
  emptyState,
  className,
}: DataTableBodyProps<T>) {
  const { t } = useTranslation();
  const visibleColumns = visibleColumnIds ? columns.filter((column) => visibleColumnIds.includes(column.id)) : columns;
  const selectable = Boolean(selectedRowKeys && onSelectedRowKeysChange);

  function toggleSort(column: DataTableColumn<T>) {
    if (!column.sortable || !onSortChange) return;
    const next: DataTableSort = sort?.columnId === column.id && sort.direction === "asc"
      ? { columnId: column.id, direction: "desc" }
      : { columnId: column.id, direction: "asc" };
    onSortChange(next);
  }

  function toggleRow(key: string) {
    if (!selectedRowKeys || !onSelectedRowKeysChange) return;
    const next = new Set(selectedRowKeys);
    if (next.has(key)) next.delete(key); else next.add(key);
    onSelectedRowKeysChange(next);
  }

  function toggleAll() {
    if (!selectedRowKeys || !onSelectedRowKeysChange) return;
    const allSelected = rows.length > 0 && rows.every((row) => selectedRowKeys.has(rowKey(row)));
    onSelectedRowKeysChange(allSelected ? new Set() : new Set(rows.map(rowKey)));
  }

  if (rows.length === 0 && emptyState) return <div className={className}>{emptyState}</div>;

  return (
    <div className={cn("flex flex-col gap-2", className)}>
      {onVisibleColumnIdsChange && visibleColumnIds ? (
        <div className="flex justify-end">
          <ColumnVisibilityMenu columns={columns} onVisibleColumnIdsChange={onVisibleColumnIdsChange} visibleColumnIds={visibleColumnIds} />
        </div>
      ) : null}
      {compact ? (
        <ul aria-label={ariaLabel} className="flex flex-col gap-2">
          {rows.map((row) => {
            const key = rowKey(row);
            return (
              <li className="ucd-card rounded-lg p-3" key={key}>
                {selectable ? (
                  <label className="mb-2 flex items-center gap-2 text-sm font-medium">
                    <input checked={selectedRowKeys?.has(key)} onChange={() => toggleRow(key)} type="checkbox" />
                    {t("workbenchUi.dataTable.selectRow")}
                  </label>
                ) : null}
                <dl className="grid grid-cols-[minmax(0,auto)_minmax(0,1fr)] gap-x-3 gap-y-1 text-sm">
                  {visibleColumns.map((column) => (
                    <div className="contents" key={column.id}>
                      <dt className="text-muted-foreground">{column.header}</dt>
                      <dd className="min-w-0">{column.cell(row)}</dd>
                    </div>
                  ))}
                </dl>
              </li>
            );
          })}
        </ul>
      ) : (
        <table aria-label={ariaLabel} className="w-full text-sm">
          <thead>
            <tr>
              {selectable ? (
                <th className="w-8 py-2 pl-3">
                  <input
                    aria-label={t("workbenchUi.dataTable.selectAll")}
                    checked={rows.length > 0 && rows.every((row) => selectedRowKeys?.has(rowKey(row)))}
                    onChange={toggleAll}
                    type="checkbox"
                  />
                </th>
              ) : null}
              {visibleColumns.map((column) => (
                <th className={cn("border-b border-border-subtle px-3 py-2 font-medium", column.align === "end" ? "text-right" : "text-left")} key={column.id}>
                  {column.sortable ? (
                    <button className="ucd-focus-ring inline-flex items-center gap-1 rounded-sm" onClick={() => toggleSort(column)} type="button">
                      {column.header}
                      <SortIcon active={sort?.columnId === column.id} direction={sort?.columnId === column.id ? sort.direction : undefined} />
                    </button>
                  ) : column.header}
                </th>
              ))}
            </tr>
          </thead>
          <tbody>
            {rows.map((row) => {
              const key = rowKey(row);
              return (
                <tr className="border-b border-border-subtle last:border-b-0" key={key}>
                  {selectable ? (
                    <td className="pl-3">
                      <input
                        aria-label={t("workbenchUi.dataTable.selectRow")}
                        checked={selectedRowKeys?.has(key)}
                        onChange={() => toggleRow(key)}
                        type="checkbox"
                      />
                    </td>
                  ) : null}
                  {visibleColumns.map((column) => (
                    <td className={cn("px-3 py-2", column.align === "end" ? "text-right" : "text-left")} key={column.id}>
                      {column.cell(row)}
                    </td>
                  ))}
                </tr>
              );
            })}
          </tbody>
        </table>
      )}
      {page && onPageChange ? <DataTablePagination onPageChange={onPageChange} page={page} /> : null}
    </div>
  );
}

function SortIcon({ active, direction }: { active?: boolean; direction?: "asc" | "desc" }) {
  if (!active) return <ArrowUpDown aria-hidden="true" className="h-3 w-3 text-muted-foreground" />;
  return direction === "asc" ? <ArrowUp aria-hidden="true" className="h-3 w-3" /> : <ArrowDown aria-hidden="true" className="h-3 w-3" />;
}

function DataTablePagination({ page, onPageChange }: { page: DataTablePage; onPageChange: (index: number) => void }) {
  const { t } = useTranslation();
  const totalPages = Math.max(1, Math.ceil(page.totalCount / page.size));
  const currentPage = page.index + 1;
  return (
    <div className="flex items-center justify-end gap-3 text-sm">
      <span className="text-muted-foreground">{t("workbenchUi.dataTable.pageOfTotal", { page: currentPage, totalPages })}</span>
      <button
        aria-label={t("workbenchUi.dataTable.previousPage")}
        className="ucd-focus-ring rounded-md border border-border px-2 py-1 disabled:opacity-50"
        disabled={page.index === 0}
        onClick={() => onPageChange(page.index - 1)}
        type="button"
      >
        {"‹"}
      </button>
      <button
        aria-label={t("workbenchUi.dataTable.nextPage")}
        className="ucd-focus-ring rounded-md border border-border px-2 py-1 disabled:opacity-50"
        disabled={currentPage >= totalPages}
        onClick={() => onPageChange(page.index + 1)}
        type="button"
      >
        ›
      </button>
    </div>
  );
}
