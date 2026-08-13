import { useEffect, useMemo, useState, type ReactNode } from "react";
import { useQuery } from "@tanstack/react-query";
import {
  Activity,
  Bot,
  Brain,
  FolderGit2,
  Gauge,
  Sparkles,
} from "lucide-react";
import { useTranslation } from "react-i18next";
import { AgentBrandIcon } from "../components/agent-brand-icon";
import { resolveModelLabel } from "../components/chat/models";
import { formatAppNumber } from "../i18n/format";
import { getAgentVisualIdentity } from "../lib/agent-visual-identity";
import { normalizeDisplayPath } from "../lib/session-path";
import { cn } from "../lib/utils";
import { agentService } from "../services/runtime-agent-client";
import { seatsFromSession } from "../services/session-seats";
import type { Session } from "../types/agent";
import type { SessionUsageSummary } from "../types/chat";
import { SessionSkillsPane } from "./session-skills-pane";
import { SessionCodeIndexPane } from "./session-code-index-pane";
import { SessionRosterEditor } from "./session-roster-editor";
import { SessionImPane } from "./session-im-pane";

export type InfoTab = "members" | "basic" | "usage" | "skills" | "im" | "codeIndex";

const tabs: Array<{ key: InfoTab; labelKey: string }> = [
  { key: "basic", labelKey: "layout.infoTab.basic" },
  { key: "usage", labelKey: "layout.infoTab.tokenUsage" },
  { key: "skills", labelKey: "layout.infoTab.skills" },
  { key: "im", labelKey: "layout.infoTab.im" },
];

function Pane({ active, children, tab }: { active: boolean; children: ReactNode; tab: InfoTab }) {
  return (
    <div
      aria-labelledby={`info-tab-${tab}`}
      className={cn("h-full", active ? "block" : "hidden")}
      data-testid={`info-pane-${tab}`}
      id={`info-pane-${tab}`}
      role="tabpanel"
    >
      {children}
    </div>
  );
}

function Field({ icon, label, value }: { icon: ReactNode; label: string; value: ReactNode }) {
  return (
    <div className="border-b border-border/60 pb-2 last:border-0 last:pb-0">
      <dt className="flex items-center gap-1.5 text-xs text-muted-foreground">
        {icon}
        <span className="truncate">{label}</span>
      </dt>
      <dd className="mt-1 min-h-5 wrap-break-word text-sm font-medium">{value}</dd>
    </div>
  );
}

function UsageMetric({ label, language, value }: { label: string; language: string; value: number }) {
  return (
    <div className="min-w-0">
      <dt className="truncate text-xs text-muted-foreground">{label}</dt>
      <dd className="mt-1 text-lg font-semibold tabular-nums text-primary">{formatAppNumber(value, language)}</dd>
    </div>
  );
}

function EmptyState({ children }: { children: ReactNode }) {
  return <p className="p-3 text-center text-xs text-muted-foreground">{children}</p>;
}

function TokenUsagePane({ loading, summary }: { loading: boolean; summary: SessionUsageSummary | undefined }) {
  const { i18n, t } = useTranslation();
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
          <span className="text-xs text-muted-foreground">{formatAppNumber(summary.coverage.reportedResponses, i18n.language)}</span>
        </div>
        {hasReported ? (
          <dl className="grid grid-cols-2 gap-2">
            <UsageMetric label={t("layout.info.usage.input")} language={i18n.language} value={summary.reported.inputTokens} />
            <UsageMetric label={t("layout.info.usage.output")} language={i18n.language} value={summary.reported.outputTokens} />
            <UsageMetric label={t("layout.info.usage.cacheRead")} language={i18n.language} value={summary.reported.cacheReadTokens} />
            <UsageMetric label={t("layout.info.usage.cacheCreation")} language={i18n.language} value={summary.reported.cacheCreationTokens} />
            <div className="col-span-2">
              <UsageMetric label={t("layout.info.usage.total")} language={i18n.language} value={summary.reported.totalTokens} />
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
            <UsageMetric label={t("layout.info.usage.estimatedResponses")} language={i18n.language} value={summary.coverage.estimatedResponses} />
            <UsageMetric label={t("layout.info.usage.totalCharacters")} language={i18n.language} value={summary.estimated.totalCharacters} />
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
  currentSpeakerSeatId = null,
  requestedTab,
  onOpenSkillSettings,
  onOpenImSettings,
}: {
  activeSession: Session | null;
  collapsed: boolean;
  currentSpeakerSeatId?: string | null;
  requestedTab?: InfoTab | null;
  onOpenSkillSettings?: () => void;
  onOpenImSettings?: () => void;
}) {
  const { t } = useTranslation();
  const [activeTab, setActiveTab] = useState<InfoTab>("basic");
  const sessionId = activeSession?.id ?? null;
  const workspacePath = activeSession?.worktreePath ?? activeSession?.projectPath ?? null;
  const workspaceDisplayPath = workspacePath ?? activeSession?.folder ?? null;
  const identity = getAgentVisualIdentity(activeSession?.agentId ?? "");
  const chatConfig = useQuery({ enabled: Boolean(sessionId), queryKey: ["session-chat-config", sessionId], queryFn: () => agentService.getSessionChatConfig(sessionId ?? "") });
  const modelLabel = useMemo(() => {
    return resolveModelLabel(chatConfig.data?.providerId, chatConfig.data?.modelId) || null;
  }, [chatConfig.data?.providerId, chatConfig.data?.modelId]);
  // While a session is running, the backend re-reads the CLI's own usage data every
  // few seconds (see TERMINAL_USAGE_POLL_INTERVAL); refetching on the same cadence
  // here is what actually surfaces those writes without waiting for the user to stop.
  const usage = useQuery({
    enabled: Boolean(sessionId),
    queryKey: ["session-usage-summary", sessionId],
    queryFn: () => agentService.getSessionUsageSummary(sessionId ?? ""),
    refetchInterval: activeSession?.lifecycleState === "running" ? 5000 : false,
  });
  const showCodeIndex = activeSession?.agentId === "onepiece" && Boolean(workspacePath);
  const showSessionMembers = Boolean(activeSession && seatsFromSession(activeSession).length > 1);
  const visibleTabs = [
    ...(showSessionMembers ? [{ key: "members" as const, labelKey: "session.memberInfo" }] : []),
    ...tabs,
    ...(showCodeIndex ? [{ key: "codeIndex" as const, labelKey: "layout.infoTab.codeIndex" }] : []),
  ];
  const tabColumns = visibleTabs.length === 6
    ? "grid-cols-6"
    : visibleTabs.length === 5
      ? "grid-cols-5"
      : "grid-cols-4";

  useEffect(() => {
    if (requestedTab && (
      (requestedTab !== "members" || showSessionMembers)
      && (requestedTab !== "codeIndex" || showCodeIndex)
    )) setActiveTab(requestedTab);
    else setActiveTab(showSessionMembers ? "members" : "basic");
  }, [requestedTab, sessionId, showCodeIndex, showSessionMembers]);

  return (
    <aside className={cn("min-w-0 overflow-hidden bg-[hsl(var(--panel-muted))] transition-[opacity,transform] duration-200 max-[900px]:absolute max-[900px]:inset-y-0 max-[900px]:right-0 max-[900px]:z-30 max-[900px]:w-[min(320px,90vw)] max-[900px]:border-l max-[900px]:border-border max-[900px]:shadow-xl", collapsed ? "pointer-events-none translate-x-2 opacity-0" : "opacity-100")}>
      <div className="flex h-full min-h-0 flex-col p-3">
        <div className="mb-3 flex h-7 items-center"><h2 className="text-sm font-semibold">{t("layout.infoPanel")}</h2></div>
        <div aria-label={t("layout.infoPanel")} className={cn("ucd-segmented mb-3 grid gap-1 rounded-md p-1", tabColumns)} role="tablist">
          {visibleTabs.map((tab) => (
            <button
              aria-controls={`info-pane-${tab.key}`}
              aria-selected={activeTab === tab.key}
              className={cn("h-8 min-w-0 truncate rounded-md px-1 text-xs", activeTab === tab.key ? "bg-background font-semibold text-primary shadow-xs" : "text-muted-foreground hover:bg-muted")}
              id={`info-tab-${tab.key}`}
              key={tab.key}
              onClick={() => setActiveTab(tab.key)}
              role="tab"
              tabIndex={activeTab === tab.key ? 0 : -1}
              title={t(tab.labelKey)}
              type="button"
            >
              {t(tab.labelKey)}
            </button>
          ))}
        </div>
        <div className="min-h-0 flex-1 overflow-y-auto pr-1">
          {showSessionMembers && activeSession ? (
            <Pane active={activeTab === "members"} tab="members"><SessionRosterEditor currentSpeakerSeatId={currentSpeakerSeatId} session={activeSession} /></Pane>
          ) : null}
          <Pane active={activeTab === "basic"} tab="basic">
            <dl className="ucd-muted-panel grid gap-2 rounded-lg p-3">
              <Field icon={<Bot className="h-3.5 w-3.5 text-primary" />} label={t("layout.info.session")} value={activeSession?.title ?? t("layout.noSession")} />
              <Field icon={<Sparkles className="h-3.5 w-3.5 text-primary" />} label={t("layout.info.cli")} value={<span className="flex min-w-0 items-center gap-2"><span className={cn("flex h-6 w-6 shrink-0 items-center justify-center rounded border", identity.tone)}><AgentBrandIcon agentId={activeSession?.agentId} className="h-3.5 w-3.5" /></span><span className="truncate">{activeSession ? identity.label : t("layout.startChat")}</span></span>} />
              <Field icon={<Activity className="h-3.5 w-3.5 text-primary" />} label={t("layout.info.lifecycle")} value={activeSession ? t(`layout.lifecycle.${activeSession.lifecycleState}`) : t("layout.noSession")} />
              <Field icon={<Brain className="h-3.5 w-3.5 text-primary" />} label={t("layout.info.model")} value={modelLabel ?? t("layout.info.modelUnavailable")} />
              <Field
                icon={<FolderGit2 className="h-3.5 w-3.5 text-primary" />}
                label={t("layout.info.workspace")}
                value={workspaceDisplayPath ? normalizeDisplayPath(workspaceDisplayPath) : t("layout.info.workspaceUnavailable")}
              />
            </dl>
          </Pane>
          <Pane active={activeTab === "usage"} tab="usage"><TokenUsagePane loading={usage.isLoading} summary={usage.data} /></Pane>
          <Pane active={activeTab === "skills"} tab="skills">
            <SessionSkillsPane activeSession={activeSession} onOpenSkillSettings={onOpenSkillSettings} />
          </Pane>
          <Pane active={activeTab === "im"} tab="im">
            <SessionImPane onOpenSettings={onOpenImSettings} sessionId={sessionId} />
          </Pane>
          {showCodeIndex && workspacePath ? <Pane active={activeTab === "codeIndex"} tab="codeIndex"><SessionCodeIndexPane workspacePath={workspacePath} /></Pane> : null}
        </div>
      </div>
    </aside>
  );
}
