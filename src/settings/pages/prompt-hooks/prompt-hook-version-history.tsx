import { useTranslation } from "react-i18next";
import { Button } from "../../../components/ui/button";
import type { PromptHookEvaluationSummary, PromptHookVersionHistory } from "../../../types/prompt-hook";

export function PromptHookVersionHistoryView({
  history,
  rollbackPending,
  onRollback,
}: {
  history: PromptHookVersionHistory | undefined;
  rollbackPending: boolean;
  onRollback: (version: number) => void;
}) {
  const { i18n, t } = useTranslation();
  return (
    <div className="space-y-3">
      {(history?.versions ?? []).map((version) => {
        const evaluation = history?.evaluations.find((item) => item.version === version.version);
        const active = history?.publishedVersion === version.version;
        return (
          <article className="rounded-md border border-border bg-[hsl(var(--panel-muted))] p-3" key={version.version}>
            <div className="flex flex-wrap items-center justify-between gap-2">
              <div className="font-mono text-sm font-semibold">
                v{version.version} {active ? `· ${t("promptHooks.lifecycle.active")}` : ""}
              </div>
              {!active ? (
                <Button disabled={rollbackPending} onClick={() => onRollback(version.version)} size="sm" variant="outline">
                  {t("promptHooks.lifecycle.rollback")}
                </Button>
              ) : null}
            </div>
            <div className="mt-1 text-xs text-muted-foreground">
              {new Intl.DateTimeFormat(i18n.language, { dateStyle: "medium", timeStyle: "short" })
                .format(new Date(version.publishedAt))}
              {version.rollbackFromVersion
                ? ` · ${t("promptHooks.lifecycle.rollbackFrom", { version: version.rollbackFromVersion })}`
                : ""}
            </div>
            <Evaluation summary={evaluation} language={i18n.language} />
          </article>
        );
      })}
      {(history?.versions.length ?? 0) === 0 ? (
        <p className="text-sm text-muted-foreground">{t("promptHooks.lifecycle.noEvaluation")}</p>
      ) : null}
      <p className="text-xs text-muted-foreground">{t("promptHooks.lifecycle.attribution")}</p>
    </div>
  );
}

function Evaluation({ summary, language }: { summary: PromptHookEvaluationSummary | undefined; language: string }) {
  const { t } = useTranslation();
  if (!summary) return <p className="mt-3 text-xs text-muted-foreground">{t("promptHooks.lifecycle.noEvaluation")}</p>;
  const percent = new Intl.NumberFormat(language, { style: "percent", maximumFractionDigits: 1 });
  const number = new Intl.NumberFormat(language, { maximumFractionDigits: 0 });
  return (
    <dl className="mt-3 grid grid-cols-2 gap-2 text-xs sm:grid-cols-4">
      <Metric label={t("promptHooks.lifecycle.successRate")} value={summary.successRate == null ? "—" : percent.format(summary.successRate)} />
      <Metric label={t("promptHooks.lifecycle.averageTime")} value={summary.averageElapsedMs == null ? "—" : `${number.format(summary.averageElapsedMs)} ms`} />
      <Metric label={t("promptHooks.lifecycle.outcomes")} value={`${summary.succeededCount}/${summary.failedCount}`} />
      <Metric label={t("promptHooks.lifecycle.cancelled")} value={number.format(summary.cancelledCount)} />
    </dl>
  );
}

function Metric({ label, value }: { label: string; value: string }) {
  return <div><dt className="text-muted-foreground">{label}</dt><dd className="mt-0.5 font-mono">{value}</dd></div>;
}
