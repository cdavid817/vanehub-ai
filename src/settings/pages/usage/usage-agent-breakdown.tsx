import { Bot } from "lucide-react";
import { useTranslation } from "react-i18next";
import type { UsageAgentBreakdown as AgentUsage } from "../../../types/chat";
import { SectionPanel } from "../page-parts";
import { formatUsageNumber } from "./usage-format";

interface UsageAgentBreakdownProps {
  agents: AgentUsage[];
  language: string;
}

export function UsageAgentBreakdown({ agents, language }: UsageAgentBreakdownProps) {
  const { t } = useTranslation();
  return (
    <SectionPanel description={t("usage.agents.description")} title={t("usage.agents.title")}>
      {agents.length === 0 ? (
        <p className="py-8 text-center text-sm text-muted-foreground">{t("usage.agents.empty")}</p>
      ) : (
        <div className="space-y-2">
          {agents.map((agent) => (
            <article className="rounded-md border border-border bg-[hsl(var(--panel-muted))] p-3" key={agent.agentId}>
              <div className="flex items-center justify-between gap-3">
                <div className="min-w-0">
                  <div className="flex items-center gap-2 font-medium">
                    <Bot className="h-4 w-4 shrink-0 text-primary" aria-hidden="true" />
                    <span className="truncate">{agent.agentId}</span>
                  </div>
                  <p className="mt-1 text-xs text-muted-foreground">
                    {t("usage.agents.responses", { count: formatUsageNumber(agent.responseCount, language) })}
                  </p>
                </div>
                <div className="shrink-0 text-right text-xs">
                  <p>{t("usage.agents.tokens", { count: formatUsageNumber(agent.reported.totalTokens, language) })}</p>
                  <p className="mt-1 text-muted-foreground">
                    {t("usage.agents.characters", { count: formatUsageNumber(agent.estimated.totalCharacters, language) })}
                  </p>
                </div>
              </div>
            </article>
          ))}
        </div>
      )}
    </SectionPanel>
  );
}
