import { useEffect, useMemo, useState, type ReactNode } from "react";
import { useQuery } from "@tanstack/react-query";
import {
  Activity,
  Bot,
  Brain,
  FolderGit2,
  Sparkles,
} from "lucide-react";
import { useTranslation } from "react-i18next";
import { AgentBrandIcon } from "../components/agent-brand-icon";
import { resolveModelLabel } from "../components/chat/models";
import { getAgentVisualIdentity } from "../lib/agent-visual-identity";
import { normalizeDisplayPath } from "../lib/session-path";
import { cn } from "../lib/utils";
import { agentService } from "../services/runtime-agent-client";
import { seatsFromSession } from "../services/session-seats";
import type { SessionTabId } from "../session-workspace/session-tab-bar";
import type { Session } from "../types/agent";
import { SessionSkillsPane } from "./session-skills-pane";
import { SessionCodeIndexPane } from "./session-code-index-pane";
import { SessionEvidenceSummary } from "./session-evidence-summary";
import { SessionRosterEditor } from "./session-roster-editor";
import { SessionImPane } from "./session-im-pane";
import { SessionTokenUsagePane } from "./session-token-usage-pane";

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

export function SessionInfoPanel({
  activeSession,
  collapsed,
  currentSpeakerSeatId = null,
  requestedTab,
  onNavigateToTab,
  onOpenSkillSettings,
  onOpenImSettings,
}: {
  activeSession: Session | null;
  collapsed: boolean;
  currentSpeakerSeatId?: string | null;
  requestedTab?: InfoTab | null;
  /** Absent where nothing owns the workspace tabs, in which case the rows are not navigable. */
  onNavigateToTab?: (tab: SessionTabId) => void;
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
            {activeSession ? (
            <>
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
            <SessionEvidenceSummary
              active={activeTab === "basic"}
              onNavigateToTab={onNavigateToTab}
              // Usage is a pane in this panel rather than a workspace tab, so its row stays here.
              onShowUsage={() => setActiveTab("usage")}
              sessionId={sessionId}
            />
            </>
            ) : (
              // Every field rendered its own "no session selected" placeholder, so an empty
              // panel repeated the same sentence five times.
              <div className="ucd-muted-panel grid gap-2 rounded-lg p-4 text-center">
                <Bot aria-hidden="true" className="mx-auto h-5 w-5 text-muted-foreground" />
                <p className="text-xs font-medium">{t("layout.noSession")}</p>
                <p className="text-[11px] leading-5 text-muted-foreground">{t("layout.startChat")}</p>
              </div>
            )}
          </Pane>
          <Pane active={activeTab === "usage"} tab="usage"><SessionTokenUsagePane active={activeTab === "usage"} lifecycle={activeSession?.lifecycleState} sessionId={sessionId} /></Pane>
          <Pane active={activeTab === "skills"} tab="skills">
            <SessionSkillsPane active={activeTab === "skills"} activeSession={activeSession} onOpenSkillSettings={onOpenSkillSettings} />
          </Pane>
          <Pane active={activeTab === "im"} tab="im">
            <SessionImPane active={activeTab === "im"} onOpenSettings={onOpenImSettings} sessionId={sessionId} />
          </Pane>
          {showCodeIndex && workspacePath ? <Pane active={activeTab === "codeIndex"} tab="codeIndex"><SessionCodeIndexPane active={activeTab === "codeIndex"} workspacePath={workspacePath} /></Pane> : null}
        </div>
      </div>
    </aside>
  );
}
