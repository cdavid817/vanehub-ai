import { useTranslation } from "react-i18next";
import { formatAppDateTime } from "../i18n/format";
import type { SystemActivityHealth } from "../services/system-activity-service";

interface SystemActivityHealthPanelProps {
  health: SystemActivityHealth;
  language: string;
}

function safeCode(value: string | null): string {
  return value ?? "—";
}

/** Keeps projector diagnostics visible without exposing source payloads or mutable controls. */
export function SystemActivityHealthPanel({ health, language }: SystemActivityHealthPanelProps) {
  const { t } = useTranslation();

  return (
    <section
      aria-label={t("systemActivity.view.health")}
      className="rounded-lg border border-border p-3 text-xs text-muted-foreground"
      data-testid="system-activity-health"
    >
      <div className="flex items-center justify-between gap-2">
        <h3 className="font-semibold text-foreground">{t("systemActivity.view.health")}</h3>
        <span className="max-w-32 truncate font-mono" title={health.leaseOwner ?? undefined}>
          {health.leaseOwner ?? t("systemActivity.view.healthLeaseIdle")}
        </span>
      </div>
      <p className="mt-1">
        {health.lastCompletedAtMs
          ? t("systemActivity.view.lastProjected", {
              time: formatAppDateTime(health.lastCompletedAtMs, language, {
                dateStyle: "medium",
                timeStyle: "short",
              }),
            })
          : t("systemActivity.view.neverProjected")}
      </p>
      {health.domains.length > 0 ? (
        <ul className="mt-2 space-y-1" data-testid="system-activity-domain-health">
          {health.domains.map((domain) => (
            <li className="rounded-md bg-muted/60 p-2" key={domain.sourceDomain}>
              <div className="flex items-center justify-between gap-2">
                <span className="truncate font-mono text-foreground">{domain.sourceDomain}</span>
                <span>{t("systemActivity.view.healthPending", { count: domain.pendingCount })}</span>
              </div>
              <p>{t("systemActivity.view.healthCursor", { sequence: domain.lastSequence })}</p>
              {domain.oldestPendingAtMs ? (
                <p>
                  {t("systemActivity.view.healthOldestPending", {
                    time: formatAppDateTime(domain.oldestPendingAtMs, language, {
                      dateStyle: "short",
                      timeStyle: "short",
                    }),
                  })}
                </p>
              ) : null}
              {domain.gap || domain.failureCode ? (
                <p className="break-all text-destructive">
                  {t("systemActivity.view.healthProblem", {
                    gap: safeCode(domain.gap),
                    failure: safeCode(domain.failureCode),
                  })}
                </p>
              ) : null}
            </li>
          ))}
        </ul>
      ) : null}
      {health.rebuilds.length > 0 ? (
        <div className="mt-2 border-t border-border pt-2" data-testid="system-activity-rebuild-history">
          <h4 className="font-medium text-foreground">{t("systemActivity.view.rebuildHistory")}</h4>
          <ul className="mt-1 space-y-1">
            {health.rebuilds.slice(0, 3).map((rebuild) => (
              <li className="flex items-center justify-between gap-2" key={rebuild.rebuildId}>
                <span className="min-w-0 truncate font-mono" title={rebuild.rebuildId}>
                  {rebuild.canonicalScopeId}
                </span>
                <span className="shrink-0">
                  {rebuild.status} · {rebuild.processedItems}/{rebuild.itemBudget}
                </span>
              </li>
            ))}
          </ul>
        </div>
      ) : null}
    </section>
  );
}
