import { useTranslation } from "react-i18next";
import { cn } from "../lib/utils";
import {
  EXECUTION_RECORD_VIEWS,
  SELECTABLE_FIDELITIES,
  SELECTABLE_STATUSES,
  toggleSelection,
  type ExecutionRecordFilterState,
  type ExecutionRecordView,
} from "./execution-record-view";

function Chip({
  isActive,
  label,
  onClick,
  testId,
}: {
  isActive: boolean;
  label: string;
  onClick: () => void;
  testId?: string;
}) {
  return (
    <button
      aria-pressed={isActive}
      className={cn(
        "h-7 shrink-0 rounded border px-2 text-xs",
        isActive ? "border-primary bg-background text-primary" : "border-border text-muted-foreground hover:bg-muted",
      )}
      data-testid={testId}
      onClick={onClick}
      type="button"
    >
      {label}
    </button>
  );
}

/**
 * The view selector and the filters, in that order.
 *
 * The view owns which kinds are asked for and the filters own everything else, so the two can
 * never contradict: there is no way to be in the Commands view with a tool filter applied, because
 * the tool filter does not exist.
 */
export function ExecutionRecordToolbar({
  filters,
  onFiltersChange,
  onViewChange,
  view,
}: {
  filters: ExecutionRecordFilterState;
  onFiltersChange: (filters: ExecutionRecordFilterState) => void;
  onViewChange: (view: ExecutionRecordView) => void;
  view: ExecutionRecordView;
}) {
  const { t } = useTranslation();
  return (
    <div className="grid gap-2">
      <div
        aria-label={t("executionRecords.viewLabel")}
        className="ucd-scroll-strip flex gap-1 overflow-x-auto"
        role="tablist"
      >
        {EXECUTION_RECORD_VIEWS.map((entry) => (
          <button
            aria-selected={view === entry}
            className={cn(
              "h-7 shrink-0 rounded px-2 text-xs",
              view === entry
                ? "bg-background font-semibold text-primary shadow-xs"
                : "text-muted-foreground hover:bg-muted",
            )}
            data-testid={`execution-record-view-${entry}`}
            key={entry}
            onClick={() => onViewChange(entry)}
            role="tab"
            type="button"
          >
            {t(`executionRecords.view.${entry}`)}
          </button>
        ))}
      </div>
      <div className="flex flex-wrap items-center gap-1">
        <span className="text-[11px] uppercase text-muted-foreground">
          {t("executionRecords.filter.status")}
        </span>
        {SELECTABLE_STATUSES.map((status) => (
          <Chip
            isActive={filters.statuses.includes(status)}
            key={status}
            label={t(`executionRecords.status.${status}`)}
            onClick={() =>
              onFiltersChange({
                ...filters,
                statuses: toggleSelection(filters.statuses, status, SELECTABLE_STATUSES),
              })
            }
            testId={`execution-record-status-${status}`}
          />
        ))}
      </div>
      <div className="flex flex-wrap items-center gap-1">
        <span className="text-[11px] uppercase text-muted-foreground">
          {t("executionRecords.filter.fidelity")}
        </span>
        {SELECTABLE_FIDELITIES.map((fidelity) => (
          <Chip
            isActive={filters.fidelities.includes(fidelity)}
            key={fidelity}
            label={t(`executionRecords.fidelity.${fidelity}`)}
            onClick={() =>
              onFiltersChange({
                ...filters,
                fidelities: toggleSelection(filters.fidelities, fidelity, SELECTABLE_FIDELITIES),
              })
            }
            testId={`execution-record-fidelity-filter-${fidelity}`}
          />
        ))}
      </div>
      <input
        aria-label={t("executionRecords.filter.search")}
        className="ucd-input h-8 w-full rounded px-2 text-sm"
        onChange={(event) => onFiltersChange({ ...filters, search: event.target.value })}
        placeholder={t("executionRecords.filter.search")}
        value={filters.search}
      />
    </div>
  );
}
