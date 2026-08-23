import { AlertTriangle } from "lucide-react";
import { useTranslation } from "react-i18next";
import { ApplicationDialog } from "../../../components/ui/application-dialog";
import { Badge } from "../../../components/ui/badge";
import { Button } from "../../../components/ui/button";
import { formatAppDateTime } from "../../../i18n/format";
import type { CliActionPlan } from "../../../types/cli-environment-snapshot";

const DATE_TIME: Intl.DateTimeFormatOptions = { timeStyle: "short" };

/** States in which a plan can no longer be run. Confirming one would be refused by the backend. */
function unusableReason(plan: CliActionPlan): string | null {
  if (plan.state === "expired") return "cli.error.plan-expired";
  if (plan.state !== "draft") return "cli.error.plan-consumed";
  return null;
}

/**
 * The review step.
 *
 * Everything the user is agreeing to is on screen, including the exact argv. Confirming submits the
 * plan id and the revision that was displayed -- nothing else, so nothing else can be substituted
 * between what was reviewed and what runs.
 */
export function CliActionPlanDialog({
  plan,
  displayName,
  submitting,
  onConfirm,
  onCancel,
  onPrepareAgain,
  returnFocus,
}: {
  plan: CliActionPlan;
  /** The registry name, from the snapshot. The locale files hold no copy of the tool catalog. */
  displayName: string;
  submitting: boolean;
  onConfirm: (input: { planId: string; expectedRevision: number }) => void;
  onCancel: () => void;
  onPrepareAgain: () => void;
  returnFocus?: HTMLElement | null;
}) {
  const { t, i18n } = useTranslation();
  const unusable = unusableReason(plan);

  return (
    <ApplicationDialog
      closeDisabled={submitting}
      description={t("cli.plan.description")}
      footer={
        <div className="flex flex-wrap justify-end gap-2">
          <Button disabled={submitting} variant="outline" onClick={onCancel}>
            {t("cli.plan.cancel")}
          </Button>
          {unusable ? (
            <Button onClick={onPrepareAgain}>{t("cli.plan.prepareAgain")}</Button>
          ) : (
            <Button
              data-dialog-autofocus=""
              disabled={submitting}
              onClick={() => onConfirm({ planId: plan.id, expectedRevision: plan.revision })}
            >
              {t(submitting ? "cli.plan.confirming" : "cli.plan.confirm")}
            </Button>
          )}
        </div>
      }
      returnFocus={returnFocus}
      title={t("cli.plan.title", { name: displayName })}
      onClose={onCancel}
    >
      <div className="space-y-4 text-sm">
        {unusable ? (
          <div className="flex gap-2 rounded-md border p-3 text-xs ucd-status-warning">
            <AlertTriangle aria-hidden="true" className="mt-0.5 h-4 w-4 shrink-0" />
            <span>{t(unusable)}</span>
          </div>
        ) : null}

        <dl className="grid gap-3 sm:grid-cols-2">
          <div>
            <dt className="text-xs text-muted-foreground">{t("cli.plan.action")}</dt>
            <dd className="mt-0.5 font-medium">{t(`cli.action.${plan.action}`)}</dd>
          </div>
          <div>
            <dt className="text-xs text-muted-foreground">{t("cli.plan.source")}</dt>
            <dd className="mt-0.5 font-medium">
              {t(`cli.source.${plan.sourceId}`)}
              {plan.channel ? ` · ${plan.channel}` : ""}
            </dd>
          </div>
          <div>
            <dt className="text-xs text-muted-foreground">{t("cli.currentVersion")}</dt>
            <dd className="mt-0.5 font-mono">{plan.currentVersion ?? t("cli.notInstalled")}</dd>
          </div>
          <div>
            <dt className="text-xs text-muted-foreground">{t("cli.plan.targetVersion")}</dt>
            <dd className="mt-0.5 font-mono">{plan.targetVersion ?? t("cli.plan.latestOnly")}</dd>
          </div>
        </dl>

        <div>
          <h4 className="text-xs font-semibold text-muted-foreground">{t("cli.plan.command")}</h4>
          {/* Argv, one argument per line. Never a shell string: there is nothing here to quote. */}
          <pre className="mt-1 overflow-auto rounded border border-border bg-[hsl(var(--panel-muted))] p-2 font-mono text-xs">
            {[plan.commandPreview.program, ...plan.commandPreview.args].join("\n")}
          </pre>
        </div>

        <div className="flex flex-wrap gap-1.5">
          {plan.requiresNetwork ? <Badge tone="muted">{t("cli.plan.requiresNetwork")}</Badge> : null}
          {plan.requiresElevation ? (
            <Badge tone="warning">{t("cli.plan.requiresElevation")}</Badge>
          ) : null}
          {/* Stated on every plan, because the absence of fallback is the point of this design. */}
          <Badge tone="muted">{t("cli.plan.noFallback")}</Badge>
        </div>

        {plan.preconditions.length > 0 ? (
          <section>
            <h4 className="text-xs font-semibold text-muted-foreground">{t("cli.plan.preconditions")}</h4>
            <ul className="mt-1 grid gap-1 text-xs">
              {plan.preconditions.map((precondition) => (
                <li key={precondition}>{t(`cli.precondition.${precondition}`)}</li>
              ))}
            </ul>
          </section>
        ) : null}

        {plan.warnings.length > 0 ? (
          <section>
            <h4 className="text-xs font-semibold text-muted-foreground">{t("cli.plan.warnings")}</h4>
            <ul className="mt-1 grid gap-1 text-xs">
              {plan.warnings.map((warning) => (
                <li className="ucd-status-warning rounded p-1.5" key={warning}>
                  {t(`cli.planWarning.${warning}`)}
                </li>
              ))}
            </ul>
          </section>
        ) : null}

        <p className="text-xs text-muted-foreground">
          {t("cli.plan.expiresAt", {
            time: formatAppDateTime(plan.expiresAt, i18n.language, DATE_TIME),
          })}
        </p>
      </div>
    </ApplicationDialog>
  );
}
