import { useTranslation } from "react-i18next";
import { Badge } from "../../../components/ui/badge";
import type { PromptHookTraceSummary } from "../../../types/prompt-hook";
import { SectionPanel } from "../page-parts";

export function PromptHookTracePanel({ traces }: { traces: PromptHookTraceSummary[] }) {
  const { i18n, t } = useTranslation();

  return (
    <SectionPanel title={t("promptHooks.trace.title")} description={t("promptHooks.trace.description")}>
      {traces.length === 0 ? (
        <div className="text-sm text-muted-foreground">{t("promptHooks.trace.empty")}</div>
      ) : (
        <div className="overflow-x-auto">
          <table className="w-full min-w-[56rem] text-left text-sm">
            <thead className="text-xs text-muted-foreground">
              <tr>
                <th className="px-2 py-2 font-medium">{t("promptHooks.trace.hookId")}</th>
                <th className="px-2 py-2 font-medium">{t("promptHooks.trace.status")}</th>
                <th className="px-2 py-2 font-medium">{t("promptHooks.trace.hash")}</th>
                <th className="px-2 py-2 font-medium">{t("promptHooks.trace.tokens")}</th>
                <th className="px-2 py-2 font-medium">{t("promptHooks.trace.agent")}</th>
                <th className="px-2 py-2 font-medium">{t("promptHooks.trace.reason")}</th>
                <th className="px-2 py-2 font-medium">{t("promptHooks.trace.timestamp")}</th>
              </tr>
            </thead>
            <tbody>
              {traces.map((trace) => (
                <tr className="border-t border-border" key={trace.id}>
                  <td className="px-2 py-2 font-mono text-xs">{trace.hookId}</td>
                  <td className="px-2 py-2"><Badge tone={trace.status === "fired" ? "success" : "muted"}>{t(`promptHooks.status.${trace.status}`)}</Badge></td>
                  <td className="px-2 py-2 font-mono text-xs">{trace.contentHash ?? "-"}</td>
                  <td className="px-2 py-2">{trace.tokenEstimate ?? 0}</td>
                  <td className="px-2 py-2">{trace.agentId ?? "-"}</td>
                  <td className="px-2 py-2">{trace.reason ?? "-"}</td>
                  <td className="px-2 py-2 text-xs">{formatTraceTime(trace.createdAt, i18n.language)}</td>
                </tr>
              ))}
            </tbody>
          </table>
        </div>
      )}
    </SectionPanel>
  );
}

function formatTraceTime(value: string, language: string) {
  const date = new Date(value);
  if (Number.isNaN(date.getTime())) return value;
  return new Intl.DateTimeFormat(language, {
    month: "2-digit",
    day: "2-digit",
    hour: "2-digit",
    minute: "2-digit",
  }).format(date);
}
