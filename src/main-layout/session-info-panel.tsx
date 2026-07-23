import { useEffect, useMemo, useState, type ReactNode } from "react";
import { useQuery } from "@tanstack/react-query";
import {
  Activity,
  Bot,
  Brain,
  FolderGit2,
  Gauge,
  Layers3,
  PanelRightClose,
  PanelRightOpen,
  Sparkles,
} from "lucide-react";
import { useTranslation } from "react-i18next";
import { AgentBrandIcon } from "../components/agent-brand-icon";
import { Button } from "../components/ui/button";
import { getAgentVisualIdentity } from "../lib/agent-visual-identity";
import { normalizeDisplayPath } from "../lib/session-path";
import { cn } from "../lib/utils";
import { agentService } from "../services/runtime-agent-client";
import type { Session } from "../types/agent";
import type { ChatMessage, SessionUsageSummary } from "../types/chat";
import type { Skill } from "../types/skill";

export type InfoTab = "basic" | "usage" | "skills";

const tabs: Array<{ key: InfoTab; labelKey: string }> = [
  { key: "basic", labelKey: "layout.infoTab.basic" },
  { key: "usage", labelKey: "layout.infoTab.tokenUsage" },
  { key: "skills", labelKey: "layout.infoTab.skills" },
];

function Pane({ active, children }: { active: boolean; children: ReactNode }) {
  return <div className={cn("h-full", active ? "block" : "hidden")}>{children}</div>;
}

function Field({ icon, label, value }: { icon: ReactNode; label: string; value: ReactNode }) {
  return (
    <div className="rounded border border-border bg-background p-2">
      <dt className="flex items-center gap-1.5 text-xs text-muted-foreground">
        {icon}
        <span className="truncate">{label}</span>
      </dt>
      <dd className="mt-1 min-h-5 break-words text-sm font-medium">{value}</dd>
    </div>
  );
}

function UsageMetric({ label, value }: { label: string; value: number }) {
  return (
    <div className="rounded border border-border bg-background p-2">
      <dt className="truncate text-xs text-muted-foreground">{label}</dt>
      <dd className="mt-1 text-lg font-semibold tabular-nums text-primary">{value.toLocaleString()}</dd>
    </div>
  );
}

function skillMatchesAgent(skill: Skill, agentId: string) {
  return skill.boundAgentIds.includes(agentId) || skill.bindings.some((binding) => binding.agentId === agentId && binding.mounted);
}

function skillKey(skill: Skill) {
  return `${skill.scope}:${skill.workspacePath ?? ""}:${skill.id}`;
}

function SkillRow({ muted, skill }: { muted?: boolean; skill: Skill }) {
  return (
    <article className={cn("rounded border border-border bg-background p-2 text-sm", muted && "opacity-55")}>
      <div className="flex min-w-0 items-center justify-between gap-2">
        <h4 className="truncate font-medium" title={skill.metadata.name}>{skill.metadata.name}</h4>
        <span className="shrink-0 rounded border border-border px-1.5 py-0.5 text-[0.68rem] uppercase text-muted-foreground">{skill.scope}</span>
      </div>
      <p className="mt-1 line-clamp-2 text-xs text-muted-foreground">{skill.metadata.description}</p>
    </article>
  );
}

function EmptyState({ children }: { children: ReactNode }) {
  return <p className="rounded border border-border bg-background p-3 text-xs text-muted-foreground">{children}</p>;
}

function summaryWithLiveReportedTokens(summary: SessionUsageSummary | undefined, sessionId: string | null, messages: ChatMessage[]): SessionUsageSummary | undefined {
  if (!summary || summary.reported.totalTokens > 0 || !sessionId) return summary;
  const reportedMessages = messages.filter((message) => message.sessionId === sessionId && message.role === "assistant" && message.status === "completed" && message.tokenUsage);
  if (reportedMessages.length === 0) return summary;
  const reported = reportedMessages.reduce((totals, message) => {
    totals.inputTokens += message.tokenUsage?.input ?? 0;
    totals.outputTokens += message.tokenUsage?.output ?? 0;
    return totals;
  }, { inputTokens: 0, outputTokens: 0, cacheReadTokens: 0, cacheCreationTokens: 0, totalTokens: 0 });
  reported.totalTokens = reported.inputTokens + reported.outputTokens;
  if (reported.totalTokens === 0) return summary;
  const totalResponses = Math.max(summary.coverage.totalResponses, reportedMessages.length);
  return {
    ...summary,
    reported,
    coverage: {
      ...summary.coverage,
      reportedResponses: Math.max(summary.coverage.reportedResponses, reportedMessages.length),
      totalResponses,
      reportedPercent: totalResponses === 0 ? 0 : (Math.max(summary.coverage.reportedResponses, reportedMessages.length) / totalResponses) * 100,
    },
    responseCount: Math.max(summary.responseCount, reportedMessages.length),
  };
}

function TokenUsagePane({ loading, summary }: { loading: boolean; summary: SessionUsageSummary | undefined }) {
  const { t } = useTranslation();
  if (loading) return <EmptyState>{t("layout.info.loading")}</EmptyState>;
  if (!summary) return <EmptyState>{t("layout.info.noUsage")}</EmptyState>;

  const hasReported = summary.coverage.reportedResponses > 0 || summary.reported.totalTokens > 0;
  const hasEstimated = summary.coverage.estimatedResponses > 0 || summary.estimated.totalCharacters > 0;

  return (
    <div className="grid gap-3">
      <section className="ucd-muted-panel rounded-lg p-3">
        <div className="mb-3 flex items-center justify-between gap-2">
          <h3 className="flex min-w-0 items-center gap-2 text-sm font-semibold">
            <Gauge className="h-4 w-4 shrink-0 text-primary" />
            <span className="truncate">{t("layout.info.usage.reported")}</span>
          </h3>
          <span className="text-xs text-muted-foreground">{summary.coverage.reportedResponses.toLocaleString()}</span>
        </div>
        {hasReported ? (
          <dl className="grid grid-cols-2 gap-2">
            <UsageMetric label={t("layout.info.usage.input")} value={summary.reported.inputTokens} />
            <UsageMetric label={t("layout.info.usage.output")} value={summary.reported.outputTokens} />
            <UsageMetric label={t("layout.info.usage.cacheRead")} value={summary.reported.cacheReadTokens} />
            <UsageMetric label={t("layout.info.usage.cacheCreation")} value={summary.reported.cacheCreationTokens} />
            <div className="col-span-2">
              <UsageMetric label={t("layout.info.usage.total")} value={summary.reported.totalTokens} />
            </div>
          </dl>
        ) : (
          <EmptyState>{t("layout.info.usage.noReported")}</EmptyState>
        )}
      </section>
      <section className="ucd-muted-panel rounded-lg p-3">
        <h3 className="mb-3 text-sm font-semibold">{t("layout.info.usage.estimated")}</h3>
        {hasEstimated ? (
          <dl className="grid grid-cols-2 gap-2">
            <UsageMetric label={t("layout.info.usage.estimatedResponses")} value={summary.coverage.estimatedResponses} />
            <UsageMetric label={t("layout.info.usage.totalCharacters")} value={summary.estimated.totalCharacters} />
          </dl>
        ) : (
          <EmptyState>{t("layout.info.usage.noEstimated")}</EmptyState>
        )}
      </section>
    </div>
  );
}

export function SessionInfoPanel({
  activeSession,
  collapsed,
  messages = [],
  onCollapsedChange,
  requestedTab,
}: {
  activeSession: Session | null;
  collapsed: boolean;
  messages?: ChatMessage[];
  onCollapsedChange: (collapsed: boolean) => void;
  requestedTab?: InfoTab | null;
}) {
  const { t } = useTranslation();
  const [activeTab, setActiveTab] = useState<InfoTab>("basic");
  const sessionId = activeSession?.id ?? null;
  const workspacePath = activeSession?.worktreePath ?? activeSession?.projectPath ?? null;
  const workspaceDisplayPath = workspacePath ?? activeSession?.folder ?? null;
  const identity = getAgentVisualIdentity(activeSession?.agentId ?? "");
  const chatConfig = useQuery({ enabled: Boolean(sessionId), queryKey: ["session-chat-config", sessionId], queryFn: () => agentService.getSessionChatConfig(sessionId ?? "") });
  const usage = useQuery({ enabled: Boolean(sessionId), queryKey: ["session-usage-summary", sessionId], queryFn: () => agentService.getSessionUsageSummary(sessionId ?? "") });
  const usageSummary = useMemo(() => summaryWithLiveReportedTokens(usage.data, sessionId, messages), [messages, sessionId, usage.data]);
  const globalSkills = useQuery({ enabled: Boolean(sessionId), queryKey: ["skills", "global", sessionId], queryFn: () => agentService.listSkills({ scope: "global" }) });
  const workspaceSkills = useQuery({ enabled: Boolean(sessionId && workspacePath), queryKey: ["skills", "workspace", workspacePath], queryFn: () => agentService.listSkills({ scope: "workspace", workspacePath }) });

  useEffect(() => {
    if (requestedTab) setActiveTab(requestedTab);
  }, [requestedTab, sessionId]);

  const skillGroups = useMemo(() => {
    const allSkills = [...(globalSkills.data?.skills ?? []), ...(workspaceSkills.data?.skills ?? [])];
    const available = allSkills.filter((skill) => activeSession ? skill.enabled && skillMatchesAgent(skill, activeSession.agentId) : false);
    return {
      available: [...new Map(available.map((skill) => [skillKey(skill), skill])).values()],
      project: workspaceSkills.data?.skills ?? [],
    };
  }, [activeSession, globalSkills.data?.skills, workspaceSkills.data?.skills]);

  return <>
    <aside className={cn("ucd-panel min-w-0 overflow-hidden rounded-lg transition-[opacity,transform] duration-200 max-[900px]:hidden", collapsed ? "pointer-events-none translate-x-2 opacity-0" : "opacity-100")}>
      <div className="flex h-full min-h-0 flex-col p-3">
        <div className="mb-3 flex items-center justify-between gap-2"><h2 className="text-sm font-semibold">{t("layout.infoPanel")}</h2><Button className="h-7 px-2 text-xs" onClick={() => onCollapsedChange(true)} variant="outline"><PanelRightClose className="h-3.5 w-3.5" />{t("layout.collapse")}</Button></div>
        <div className="ucd-segmented mb-3 grid grid-cols-3 gap-1 rounded-md p-1">{tabs.map((tab) => <button aria-pressed={activeTab === tab.key} className={cn("h-8 truncate rounded-md px-1 text-xs", activeTab === tab.key ? "bg-background font-semibold text-primary shadow-sm" : "text-muted-foreground hover:bg-muted")} key={tab.key} onClick={() => setActiveTab(tab.key)} title={t(tab.labelKey)} type="button">{t(tab.labelKey)}</button>)}</div>
        <div className="min-h-0 flex-1 overflow-y-auto pr-1">
          <Pane active={activeTab === "basic"}>
            <dl className="grid gap-3">
              <section className="ucd-muted-panel grid gap-2 rounded-lg p-3">
                <Field icon={<Bot className="h-3.5 w-3.5 text-primary" />} label={t("layout.info.session")} value={activeSession?.title ?? t("layout.noSession")} />
                <Field icon={<Sparkles className="h-3.5 w-3.5 text-primary" />} label={t("layout.info.cli")} value={<span className="flex min-w-0 items-center gap-2"><span className={cn("flex h-6 w-6 shrink-0 items-center justify-center rounded border", identity.tone)}><AgentBrandIcon agentId={activeSession?.agentId} className="h-3.5 w-3.5" /></span><span className="truncate">{activeSession ? identity.label : t("layout.startChat")}</span></span>} />
                <Field icon={<Activity className="h-3.5 w-3.5 text-primary" />} label={t("layout.info.lifecycle")} value={activeSession ? t(`layout.lifecycle.${activeSession.lifecycleState}`) : t("layout.noSession")} />
                <Field icon={<Brain className="h-3.5 w-3.5 text-primary" />} label={t("layout.info.model")} value={chatConfig.data?.modelId ?? t("layout.info.modelUnavailable")} />
                <Field
                  icon={<FolderGit2 className="h-3.5 w-3.5 text-primary" />}
                  label={t("layout.info.workspace")}
                  value={workspaceDisplayPath ? normalizeDisplayPath(workspaceDisplayPath) : t("layout.info.workspaceUnavailable")}
                />
              </section>
            </dl>
          </Pane>
          <Pane active={activeTab === "usage"}><TokenUsagePane loading={usage.isLoading} summary={usageSummary} /></Pane>
          <Pane active={activeTab === "skills"}>
            <div className="grid gap-3">
              <section className="ucd-muted-panel rounded-lg p-3">
                <h3 className="mb-2 flex items-center gap-2 text-sm font-semibold"><Layers3 className="h-4 w-4 text-primary" />{t("layout.info.skills.available")}</h3>
                <div className="grid gap-2">{skillGroups.available.map((skill) => <SkillRow key={skillKey(skill)} skill={skill} />)}{globalSkills.isLoading || workspaceSkills.isLoading ? <EmptyState>{t("layout.info.loading")}</EmptyState> : null}{!globalSkills.isLoading && !workspaceSkills.isLoading && skillGroups.available.length === 0 ? <EmptyState>{t("layout.info.skills.noAvailable")}</EmptyState> : null}</div>
              </section>
              <section className="ucd-muted-panel rounded-lg p-3">
                <h3 className="mb-2 text-sm font-semibold">{t("layout.info.skills.project")}</h3>
                <div className="grid gap-2">{skillGroups.project.map((skill) => <SkillRow key={skillKey(skill)} muted={!skill.enabled} skill={skill} />)}{!workspacePath ? <EmptyState>{t("layout.info.skills.noWorkspace")}</EmptyState> : null}{workspacePath && !workspaceSkills.isLoading && skillGroups.project.length === 0 ? <EmptyState>{t("layout.info.skills.noProject")}</EmptyState> : null}</div>
              </section>
            </div>
          </Pane>
        </div>
      </div>
    </aside>
    {collapsed ? <Button className="absolute right-2 top-1/2 h-9 w-9 -translate-y-1/2 px-0" onClick={() => onCollapsedChange(false)} size="icon" title={t("layout.expandInfo")} variant="outline"><PanelRightOpen className="h-4 w-4" /></Button> : null}
  </>;
}
