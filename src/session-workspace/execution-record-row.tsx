import { useTranslation } from "react-i18next";
import { cn } from "../lib/utils";
import type { ExecutionRecord } from "../types/session-workspace-evidence";
import {
  absenceKey,
  durationField,
  exitField,
  fidelityKey,
  recordLabel,
  statusKey,
  type RecordField,
} from "./execution-record-fields";

/**
 * Renders one field, including the case where there is nothing to render.
 *
 * An absent field states why it is absent rather than falling back to a dash: "not observed",
 * "unavailable", and "redacted" are three different things a reader would otherwise have to guess
 * between, and one of them means the work may not have happened the way the row implies.
 */
export function FieldValue({ field }: { field: RecordField }) {
  const { t } = useTranslation();
  if (field.kind === "text") return <>{field.value}</>;
  if (field.kind === "i18n") return <>{t(field.key, field.values)}</>;
  return <span className="italic text-muted-foreground">{t(absenceKey(field.reason))}</span>;
}

const STATUS_TONE: Record<string, string> = {
  failed: "border-destructive text-destructive",
  cancelled: "border-destructive text-destructive",
  incomplete: "ucd-status-warning",
};

export function ExecutionRecordRow({
  isSelected,
  onSelect,
  record,
}: {
  isSelected: boolean;
  onSelect: (record: ExecutionRecord) => void;
  record: ExecutionRecord;
}) {
  const { t } = useTranslation();
  return (
    <button
      aria-current={isSelected ? "true" : undefined}
      className={cn(
        "flex w-full flex-col gap-1 rounded-md border px-2 py-2 text-left text-xs",
        isSelected ? "border-primary bg-background" : "border-border hover:bg-muted",
      )}
      data-record-id={record.id}
      data-testid="execution-record-row"
      onClick={() => onSelect(record)}
      type="button"
    >
      <span className="flex items-center gap-2">
        <span className="shrink-0 rounded-full border border-border px-1.5 py-0.5 text-[10px] uppercase text-muted-foreground">
          {t(`executionRecords.kind.${record.kind}`)}
        </span>
        <span className="min-w-0 flex-1 truncate font-mono">
          <FieldValue field={recordLabel(record)} />
        </span>
        <span
          className={cn(
            "shrink-0 rounded-full border px-1.5 py-0.5 text-[10px]",
            STATUS_TONE[record.status] ?? "border-border text-muted-foreground",
          )}
        >
          {t(statusKey(record))}
        </span>
      </span>
      <span className="flex flex-wrap items-center gap-x-3 gap-y-1 text-[11px] text-muted-foreground">
        <span>
          {t("executionRecords.field.duration")}: <FieldValue field={durationField(record)} />
        </span>
        {record.kind === "command" ? (
          <span>
            {t("executionRecords.field.exit")}: <FieldValue field={exitField(record)} />
          </span>
        ) : null}
        <span data-testid={`execution-record-fidelity-${record.fidelity}`}>
          {t(fidelityKey(record))}
        </span>
        {record.kind === "legacy" ? (
          <span data-testid="execution-record-legacy-source">
            {t("executionRecords.legacy.rowSource")}
          </span>
        ) : null}
      </span>
    </button>
  );
}
