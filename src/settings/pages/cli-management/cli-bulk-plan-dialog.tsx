import { useTranslation } from "react-i18next";
import { ApplicationDialog } from "../../../components/ui/application-dialog";
import { Badge } from "../../../components/ui/badge";
import { Button } from "../../../components/ui/button";
import type { CliBulkItemResult } from "../../../types/cli-environment";
import type { CliBulkActionPlan } from "../../../types/cli-environment-snapshot";

/**
 * The batch review, and the batch result once it has run.
 *
 * Two lists before execution -- what will run, and what will not with a reason each -- because a
 * shorter list with no explanation reads as "everything else is up to date".
 *
 * After execution the same rows carry the real per-item outcome. Every tool the batch knew about
 * appears; a missing row would read as "nothing to report", which is never true of a tool the user
 * asked to upgrade.
 */
export function CliBulkPlanDialog({
  plan,
  results,
  displayNames,
  submitting,
  onConfirm,
  onClose,
  returnFocus,
}: {
  plan: CliBulkActionPlan;
  results: readonly CliBulkItemResult[] | null;
  /**
   * Registry names by agent id, from the snapshots. Not a translation key: the tool catalog lives
   * on the backend, and a second copy in the locale files drifts the first time a tool is renamed.
   */
  displayNames: Readonly<Record<string, string>>;
  submitting: boolean;
  onConfirm: (input: { planId: string; expectedRevision: number }) => void;
  onClose: () => void;
  returnFocus?: HTMLElement | null;
}) {
  const { t } = useTranslation();
  const nameOf = (agentId: string) => displayNames[agentId] ?? agentId;

  return (
    <ApplicationDialog
      closeDisabled={submitting}
      description={t("cli.bulk.description")}
      footer={
        <div className="flex flex-wrap justify-end gap-2">
          <Button disabled={submitting} variant="outline" onClick={onClose}>
            {t(results ? "cli.bulk.close" : "cli.plan.cancel")}
          </Button>
          {results ? null : (
            <Button
              data-dialog-autofocus=""
              disabled={submitting || plan.items.length === 0}
              onClick={() => onConfirm({ planId: plan.id, expectedRevision: plan.revision })}
            >
              {submitting
                ? t("cli.bulk.running")
                : t("cli.bulk.confirm", { count: plan.items.length })}
            </Button>
          )}
        </div>
      }
      returnFocus={returnFocus}
      title={t("cli.bulk.title")}
      onClose={onClose}
    >
      <div className="space-y-4 text-sm">
        {results ? (
          <section>
            <h4 className="text-xs font-semibold text-muted-foreground">{t("cli.bulk.results")}</h4>
            <ul className="mt-2 grid gap-2">
              {results.map((item) => (
                <li
                  className="flex flex-wrap items-center gap-2 rounded-md border border-border p-2 text-xs"
                  key={item.agentId}
                >
                  <span className="font-medium">{nameOf(item.agentId)}</span>
                  {item.status === "completed" ? (
                    <Badge tone={item.outcome === "verified" ? "success" : "warning"}>
                      {t(`cli.outcome.${item.outcome}`)}
                    </Badge>
                  ) : (
                    <Badge tone="muted">{t(`cli.skip.${item.reason}`)}</Badge>
                  )}
                  {item.targetVersion ? (
                    <span className="ml-auto font-mono text-muted-foreground">
                      {item.targetVersion}
                    </span>
                  ) : null}
                </li>
              ))}
            </ul>
          </section>
        ) : (
          <>
            <section>
              <h4 className="text-xs font-semibold text-muted-foreground">
                {t("cli.bulk.willRun", { count: plan.items.length })}
              </h4>
              {plan.items.length === 0 ? (
                <p className="mt-1 text-xs text-muted-foreground">{t("cli.bulk.nothingToRun")}</p>
              ) : (
                <ul className="mt-2 grid gap-2">
                  {plan.items.map((item) => (
                    <li className="rounded-md border border-border p-2 text-xs" key={item.agentId}>
                      <div className="flex flex-wrap items-center gap-2">
                        <span className="font-medium">{nameOf(item.agentId)}</span>
                        <Badge tone="muted">{t(`cli.source.${item.sourceId}`)}</Badge>
                        <span className="ml-auto font-mono text-muted-foreground">
                          {`${item.currentVersion ?? t("cli.notInstalled")} → ${item.targetVersion ?? t("cli.plan.latestOnly")}`}
                        </span>
                      </div>
                    </li>
                  ))}
                </ul>
              )}
            </section>

            {plan.skipped.length > 0 ? (
              <section>
                <h4 className="text-xs font-semibold text-muted-foreground">
                  {t("cli.bulk.skipped", { count: plan.skipped.length })}
                </h4>
                <ul className="mt-2 grid gap-2">
                  {plan.skipped.map((skip) => (
                    <li
                      className="flex flex-wrap items-center gap-2 rounded-md border border-border p-2 text-xs"
                      key={skip.agentId}
                    >
                      <span className="font-medium">{nameOf(skip.agentId)}</span>
                      <Badge tone="muted">{t(`cli.skip.${skip.reason}`)}</Badge>
                    </li>
                  ))}
                </ul>
              </section>
            ) : null}
          </>
        )}
      </div>
    </ApplicationDialog>
  );
}
