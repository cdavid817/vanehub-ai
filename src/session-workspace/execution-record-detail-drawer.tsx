import { X } from "lucide-react";
import { useEffect, useRef } from "react";
import { useTranslation } from "react-i18next";
import type { ExecutionRecord } from "../types/session-workspace-evidence";
import { executionRecordActions } from "./execution-record-actions";
import {
  cwdField,
  durationField,
  endedAtField,
  exitField,
  fidelityKey,
  outputField,
  recordLabel,
  startedAtField,
  statusKey,
  verificationCountsField,
} from "./execution-record-fields";
import { FieldValue } from "./execution-record-row";
import { useWorkspaceEvidenceScope } from "./workspace-evidence-scope";

/**
 * The fields a drawer is allowed to show, named one by one.
 *
 * Every entry is a field of the record DTO, chosen deliberately. There is no branch that walks an
 * attribute map or renders a payload as JSON: a generic renderer shows whatever a producer sends,
 * which is how a raw argument vector, a stack trace, or a snippet of a diff reaches a panel that
 * was only ever meant to show identifiers and outcomes.
 */
const DETAIL_FIELDS = [
  { key: "startedAt", read: startedAtField },
  { key: "endedAt", read: endedAtField },
  { key: "duration", read: durationField },
  { key: "exit", read: exitField },
  { key: "cwd", read: cwdField },
  { key: "output", read: outputField },
  { key: "verificationCounts", read: verificationCountsField },
] as const;

export function ExecutionRecordDetailDrawer({
  onClose,
  record,
}: {
  onClose: () => void;
  record: ExecutionRecord;
}) {
  const { t } = useTranslation();
  const { navigate } = useWorkspaceEvidenceScope();
  const closeRef = useRef<HTMLButtonElement>(null);
  const actions = executionRecordActions(record);

  useEffect(() => {
    // The drawer takes focus when it opens so a keyboard reader is not left on a row behind it,
    // and the close button is the one control every drawer has.
    closeRef.current?.focus();
  }, [record.id]);

  return (
    <section
      aria-label={t("executionRecords.detail.title")}
      className="flex h-full min-h-0 flex-col gap-2 overflow-y-auto rounded-lg border border-border bg-[hsl(var(--panel-muted))] p-3"
      data-testid="execution-record-detail"
      onKeyDown={(event) => {
        if (event.key === "Escape") onClose();
      }}
      role="region"
    >
      <header className="flex items-start justify-between gap-2">
        <div className="min-w-0">
          <p className="text-[11px] uppercase text-muted-foreground">
            {t(`executionRecords.kind.${record.kind}`)}
          </p>
          <h3 className="truncate font-mono text-sm font-semibold">
            <FieldValue field={recordLabel(record)} />
          </h3>
        </div>
        <button
          aria-label={t("executionRecords.detail.close")}
          className="shrink-0 rounded border border-border p-1 hover:bg-muted"
          onClick={onClose}
          ref={closeRef}
          type="button"
        >
          <X aria-hidden="true" className="h-3.5 w-3.5" />
        </button>
      </header>

      <dl className="grid gap-1 text-xs">
        <div className="flex justify-between gap-3">
          <dt className="text-muted-foreground">{t("executionRecords.detail.status")}</dt>
          <dd>{t(statusKey(record))}</dd>
        </div>
        <div className="flex justify-between gap-3">
          <dt className="text-muted-foreground">{t("executionRecords.detail.fidelity")}</dt>
          <dd>{t(fidelityKey(record))}</dd>
        </div>
        {DETAIL_FIELDS.map(({ key, read }) => {
          const field = read(record);
          if (field.kind === "absent" && field.reason === "not-applicable") return null;
          return (
            <div className="flex justify-between gap-3" key={key}>
              <dt className="text-muted-foreground">{t(`executionRecords.detail.${key}`)}</dt>
              <dd className="min-w-0 truncate text-right">
                <FieldValue field={field} />
              </dd>
            </div>
          );
        })}
        {record.kind === "command" && record.outputTruncated ? (
          <p className="text-muted-foreground" data-testid="execution-record-output-truncated">
            {t("executionRecords.detail.outputTruncated")}
          </p>
        ) : null}
      </dl>

      {actions.length === 0 ? null : (
        <div className="flex flex-wrap gap-1" data-testid="execution-record-actions">
          {actions.map((action) => (
            <button
              className="h-7 rounded border border-border px-2 text-xs hover:bg-muted"
              data-testid={`execution-record-action-${action.id}`}
              key={action.id}
              onClick={() => navigate(action.target)}
              type="button"
            >
              {t(`executionRecords.action.${action.id}`)}
            </button>
          ))}
        </div>
      )}
    </section>
  );
}
