import { useMemo } from "react";
import { useTranslation } from "react-i18next";
import type { ChatMessage, MessageStatus } from "../types/chat";
import { aggregateSessionReport } from "./report-utils";
import { PartialNotice, WorkspaceState } from "./workspace-state";

const messageStatuses: MessageStatus[] = ["pending", "streaming", "completed", "failed", "cancelled"];

export function ReportTab({ messages, partial }: { messages: ChatMessage[]; partial: boolean }) {
  const { i18n, t } = useTranslation();
  const report = useMemo(() => aggregateSessionReport(messages), [messages]);
  if (messages.length === 0) return <WorkspaceState kind="empty" message={t("sessionTabs.report.empty")} />;
  const number = new Intl.NumberFormat(i18n.language);
  const totalTokens = report.reportedInputTokens + report.reportedOutputTokens;
  const inputWidth = totalTokens === 0 ? 0 : (report.reportedInputTokens / totalTokens) * 100;

  return (
    <div className="grid gap-4 overflow-y-auto pr-1">
      {partial ? <PartialNotice /> : null}
      <section className="grid gap-3 sm:grid-cols-2 lg:grid-cols-4">
        <Metric label={t("sessionTabs.report.inputTokens")} value={number.format(report.reportedInputTokens)} />
        <Metric label={t("sessionTabs.report.outputTokens")} value={number.format(report.reportedOutputTokens)} />
        <Metric label={t("sessionTabs.report.inputCharacters")} value={number.format(report.estimatedInputCharacters)} />
        <Metric label={t("sessionTabs.report.outputCharacters")} value={number.format(report.estimatedOutputCharacters)} />
      </section>
      <section className="rounded-lg border border-border bg-[hsl(var(--panel-muted))] p-3">
        <h3 className="mb-3 text-sm font-semibold">{t("sessionTabs.report.tokenDistribution")}</h3>
        <svg aria-label={t("sessionTabs.report.tokenDistribution")} className="h-8 w-full" role="img" viewBox="0 0 100 12">
          <rect className="fill-muted" height="12" rx="3" width="100" x="0" y="0" />
          <rect className="fill-primary" height="12" rx="3" width={inputWidth} x="0" y="0" />
        </svg>
      </section>
      <section className="rounded-lg border border-border bg-[hsl(var(--panel-muted))] p-3">
        <h3 className="mb-3 text-sm font-semibold">{t("sessionTabs.report.statusCounts")}</h3>
        <div className="grid grid-cols-2 gap-2 sm:grid-cols-5">
          {messageStatuses.map((status) => (
            <Metric
              key={status}
              label={t(`sessionTabs.report.status.${status}`)}
              value={number.format(report.statusCounts[status])}
            />
          ))}
        </div>
      </section>
      <section className="grid gap-4 lg:grid-cols-2">
        <div className="rounded-lg border border-border bg-[hsl(var(--panel-muted))] p-3">
          <h3 className="mb-3 text-sm font-semibold">{t("sessionTabs.report.toolRanking")}</h3>
          <div className="grid gap-2">
            {report.toolRanking.length === 0 ? <p className="text-sm text-muted-foreground">{t("sessionTabs.report.noTools")}</p> : report.toolRanking.map((tool) => (
              <div className="flex items-center justify-between rounded border border-border px-2 py-1.5 text-sm" key={tool.name}>
                <span className="truncate font-mono">{tool.name}</span><strong>{number.format(tool.count)}</strong>
              </div>
            ))}
          </div>
        </div>
        <div className="rounded-lg border border-border bg-[hsl(var(--panel-muted))] p-3">
          <h3 className="mb-3 text-sm font-semibold">{t("sessionTabs.report.timeline")}</h3>
          <div className="grid max-h-72 gap-2 overflow-y-auto">
            {report.timeline.map((item) => (
              <div className="flex items-center justify-between gap-3 border-l-2 border-primary pl-2 text-xs" key={item.id}>
                <span className="truncate">{t(`sessionTabs.timeline.${item.kind}`)} · {item.label}</span>
                <time className="shrink-0 text-muted-foreground">{new Intl.DateTimeFormat(i18n.language, { timeStyle: "medium" }).format(new Date(item.timestamp))}</time>
              </div>
            ))}
          </div>
        </div>
      </section>
    </div>
  );
}

function Metric({ label, value }: { label: string; value: string }) {
  return <div className="rounded-lg border border-border bg-[hsl(var(--panel-muted))] p-3"><p className="text-xs text-muted-foreground">{label}</p><strong className="mt-1 block text-xl text-primary">{value}</strong></div>;
}
