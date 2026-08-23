import { useTranslation } from "react-i18next";
import { Badge } from "../../../components/ui/badge";
import { Button } from "../../../components/ui/button";
import type { CliMutationResult } from "../../../types/cli-environment";
import type { OperationTask } from "../../../types/operation";

/** Whether an operation is still going. Queued counts: the work is accepted and not finished. */
export function isOperationRunning(operation?: OperationTask) {
  return operation?.status === "running" || operation?.status === "queued";
}

/** The typed result a finished CLI mutation carries, when it carries one. */
export function mutationResultOf(operation?: OperationTask): CliMutationResult | null {
  const result = operation?.result;
  if (!result || typeof result !== "object" || Array.isArray(result)) return null;
  return "outcome" in result ? (result as unknown as CliMutationResult) : null;
}

/**
 * The guidance an outcome needs beyond its own name.
 *
 * `applied-unverified` and `changed-but-failed` are the two the user cannot act on without being
 * told what they mean: the command ran, and the machine may not be where the label suggests.
 * Nothing here claims a rollback, because none happened.
 */
function guidanceKey(outcome: CliMutationResult["outcome"]): string | null {
  switch (outcome) {
    case "applied-unverified":
      return "cli.outcome.guidance.applied-unverified";
    case "changed-but-failed":
      return "cli.outcome.guidance.changed-but-failed";
    case "no-change-failed":
      return "cli.outcome.guidance.no-change-failed";
    default:
      return null;
  }
}

export function CliOperationStatus({
  operation,
  onCancel,
}: {
  operation: OperationTask;
  onCancel?: () => void;
}) {
  const { t } = useTranslation();
  const running = isOperationRunning(operation);
  const result = mutationResultOf(operation);
  const guidance = result ? guidanceKey(result.outcome) : null;

  return (
    <div className="rounded-md border border-border p-3 text-xs">
      <div className="flex flex-wrap items-center gap-2">
        <Badge tone={running ? "muted" : operation.status === "succeeded" ? "success" : "warning"}>
          {t(`cli.operationStatus.${operation.status}`)}
        </Badge>
        {operation.phase ? (
          <span className="text-muted-foreground">{t(`cli.phase.${operation.phase}`)}</span>
        ) : null}
        {result?.outcome ? (
          <Badge tone={result.warning ? "warning" : "success"}>
            {t(`cli.outcome.${result.outcome}`)}
          </Badge>
        ) : null}
        {running && operation.cancellable && onCancel ? (
          <Button className="ml-auto" size="sm" variant="outline" onClick={onCancel}>
            {t("cli.operation.cancel")}
          </Button>
        ) : null}
      </div>
      {guidance ? <p className="mt-2 ucd-status-warning rounded p-2">{t(guidance)}</p> : null}
      {result?.warnings.map((warning) => (
        <p className="mt-1 text-muted-foreground" key={warning}>
          {t(`cli.verificationWarning.${warning}`)}
        </p>
      ))}
    </div>
  );
}
