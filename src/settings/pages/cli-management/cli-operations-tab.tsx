import { ClipboardCopy } from "lucide-react";
import { useTranslation } from "react-i18next";
import { Button } from "../../../components/ui/button";
import { formatAppDateTime } from "../../../i18n/format";
import type { OperationTask } from "../../../types/operation";
import { CliOperationStatus, mutationResultOf } from "./cli-operation-status";

const DATE_TIME: Intl.DateTimeFormatOptions = { dateStyle: "medium", timeStyle: "medium" };

/**
 * The operation behind this tool, its typed outcome, and its bounded log.
 *
 * The log is rendered as a plain block rather than announced: a screen reader reading every line of
 * streaming output as it arrives makes the page unusable. The status line above it carries the
 * announcement, once per change.
 */
export function CliOperationsTab({
  operation,
  onCancel,
}: {
  operation?: OperationTask;
  onCancel: () => void;
}) {
  const { t, i18n } = useTranslation();

  if (!operation) {
    return <p className="text-sm text-muted-foreground">{t("cli.operations.none")}</p>;
  }

  const result = mutationResultOf(operation);
  // Identifiers, enums and timings only. The log is separate, and already redacted upstream.
  const safeSummary = [
    `operation: ${operation.id}`,
    `status: ${operation.status}`,
    operation.phase ? `phase: ${operation.phase}` : null,
    result ? `action: ${result.action ?? "-"}` : null,
    result ? `source: ${result.sourceId ?? "-"}` : null,
    result ? `outcome: ${result.outcome ?? "-"}` : null,
    result ? `termination: ${result.termination}` : null,
    result ? `elapsedMs: ${result.elapsedMs}` : null,
  ]
    .filter(Boolean)
    .join("\n");

  return (
    <div className="space-y-3 text-sm">
      <CliOperationStatus operation={operation} onCancel={onCancel} />

      <dl className="grid gap-2 text-xs sm:grid-cols-2">
        <div className="min-w-0">
          <dt className="text-muted-foreground">{t("cli.operations.id")}</dt>
          <dd className="truncate font-mono">{operation.id}</dd>
        </div>
        <div>
          <dt className="text-muted-foreground">{t("cli.operations.updatedAt")}</dt>
          <dd>{operation.updatedAt ? formatAppDateTime(operation.updatedAt, i18n.language, DATE_TIME) : "-"}</dd>
        </div>
        {result ? (
          <>
            <div>
              <dt className="text-muted-foreground">{t("cli.operations.action")}</dt>
              <dd>
                {result.action ? t(`cli.action.${result.action}`) : "-"}
                {result.sourceId ? ` · ${t(`cli.source.${result.sourceId}`)}` : ""}
              </dd>
            </div>
            <div>
              <dt className="text-muted-foreground">{t("cli.operations.termination")}</dt>
              <dd>
                {t(`cli.termination.${result.termination}`)}
                {result.exitCode !== null ? ` (${result.exitCode})` : ""}
              </dd>
            </div>
          </>
        ) : null}
      </dl>

      <div>
        <div className="flex items-center justify-between gap-2">
          <h4 className="text-xs font-semibold text-muted-foreground">{t("cli.operations.logs")}</h4>
          <Button
            size="sm"
            variant="outline"
            onClick={() => void navigator.clipboard?.writeText(safeSummary)}
          >
            <ClipboardCopy aria-hidden="true" className="h-3.5 w-3.5" />
            {t("cli.operations.copySummary")}
          </Button>
        </div>
        <div className="mt-2 max-h-48 overflow-auto rounded border border-border bg-[hsl(var(--panel-muted))] p-2 font-mono text-xs">
          {operation.logs.length === 0 ? <div>{t("cli.noLogs")}</div> : null}
          {operation.logs.map((log, index) => (
            <div className="whitespace-pre-wrap" key={`${log.timestamp}-${index}`}>{log.line}</div>
          ))}
          {operation.error ? (
            <div className="mt-2 text-[hsl(var(--danger))]">{operation.error}</div>
          ) : null}
        </div>
        {result?.outputTruncated ? (
          <p className="mt-1 text-xs text-muted-foreground">{t("cli.operations.truncated")}</p>
        ) : null}
      </div>
    </div>
  );
}
