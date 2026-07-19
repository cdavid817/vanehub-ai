import { useState, type ReactNode } from "react";
import { useQuery } from "@tanstack/react-query";
import { CheckCircle2, CircleDot, Clock3, FileText, GitBranch, PanelRightClose, PanelRightOpen } from "lucide-react";
import { useTranslation } from "react-i18next";
import { AgentBrandIcon } from "../components/agent-brand-icon";
import { Button } from "../components/ui/button";
import { getAgentVisualIdentity } from "../lib/agent-visual-identity";
import { cn } from "../lib/utils";
import { agentService } from "../services/runtime-agent-client";
import type { Session } from "../types/agent";

type InfoTab = "agent" | "files" | "changes";
const tabs: Array<{ key: InfoTab; labelKey: string }> = [
  { key: "agent", labelKey: "layout.infoTab.agent" }, { key: "files", labelKey: "layout.infoTab.files" }, { key: "changes", labelKey: "layout.infoTab.changes" },
];
function Pane({ active, children }: { active: boolean; children: ReactNode }) { return <div className={cn("h-full", active ? "block" : "hidden")}>{children}</div>; }

export function SessionInfoPanel({ activeSession, collapsed, onCollapsedChange }: { activeSession: Session | null; collapsed: boolean; onCollapsedChange: (collapsed: boolean) => void }) {
  const { t } = useTranslation();
  const [activeTab, setActiveTab] = useState<InfoTab>("agent");
  const sessionId = activeSession?.id ?? null;
  const files = useQuery({ enabled: Boolean(sessionId), queryKey: ["session-workspace", "directory", sessionId, ""], queryFn: () => agentService.listSessionDirectory(sessionId ?? "", "") });
  const changes = useQuery({ enabled: Boolean(sessionId), queryKey: ["session-workspace", "git-status", sessionId], queryFn: () => agentService.getSessionGitStatus(sessionId ?? "") });
  const progress = { complete: 6, running: 3, pending: 4 };
  const identity = getAgentVisualIdentity(activeSession?.agentId ?? "");
  return <>
    <aside className={cn("ucd-panel min-w-0 overflow-hidden rounded-lg transition-[opacity,transform] duration-200 max-[900px]:hidden", collapsed ? "pointer-events-none translate-x-2 opacity-0" : "opacity-100")}>
      <div className="flex h-full min-h-0 flex-col p-3">
        <div className="mb-3 flex items-center justify-between gap-2"><h2 className="text-sm font-semibold">{t("layout.infoPanel")}</h2><Button className="h-7 px-2 text-xs" onClick={() => onCollapsedChange(true)} variant="outline"><PanelRightClose className="h-3.5 w-3.5" />{t("layout.collapse")}</Button></div>
        <div className="ucd-segmented mb-3 grid grid-cols-3 gap-1 rounded-md p-1">{tabs.map((tab) => <button className={cn("h-8 rounded-md text-xs", activeTab === tab.key ? "bg-background font-semibold text-primary shadow-sm" : "text-muted-foreground hover:bg-muted")} key={tab.key} onClick={() => setActiveTab(tab.key)} type="button">{t(tab.labelKey)}</button>)}</div>
        <div className="min-h-0 flex-1 overflow-y-auto pr-1">
          <Pane active={activeTab === "agent"}><div className="grid gap-4"><section className="ucd-muted-panel rounded-lg p-3"><div className="mb-3 flex justify-between"><h3 className="text-sm font-semibold">{t("layout.taskProgress")}</h3><strong className="text-primary">46%</strong></div><div className="h-2 rounded bg-muted"><div className="h-2 w-[46%] rounded bg-primary" /></div><div className="mt-3 grid grid-cols-3 gap-2 text-center text-xs"><div className="rounded border border-border p-2"><CheckCircle2 className="mx-auto h-4 w-4 text-[hsl(var(--success))]" />{progress.complete}<br />{t("layout.completed")}</div><div className="rounded border border-border p-2"><CircleDot className="mx-auto h-4 w-4 text-primary" />{progress.running}<br />{t("layout.running")}</div><div className="rounded border border-border p-2"><Clock3 className="mx-auto h-4 w-4" />{progress.pending}<br />{t("layout.pending")}</div></div></section><section className="ucd-muted-panel grid gap-3 rounded-lg p-3 text-sm"><h3 className="font-semibold">{t("layout.sessionConfig")}</h3><span className="truncate rounded border border-border bg-background p-2">{activeSession?.title ?? t("layout.noSession")}</span><span className="flex min-w-0 items-center gap-2 rounded border border-border bg-background p-2"><span className={cn("flex h-7 w-7 shrink-0 items-center justify-center rounded border", identity.tone)}><AgentBrandIcon agentId={activeSession?.agentId} className="h-4 w-4" /></span><span className="min-w-0 truncate">{activeSession ? `${identity.label} · ${activeSession.interactionMode}` : t("layout.startChat")}</span></span></section></div></Pane>
          <Pane active={activeTab === "files"}><div className="grid gap-2">{files.data?.items.slice(0, 8).map((file) => <div className="flex items-center gap-2 rounded border border-border p-2 text-sm" key={file.path}><FileText className="h-4 w-4 text-primary" /><span className="truncate">{file.path}</span></div>)}{!files.isLoading && !files.data?.items.length ? <p className="text-xs text-muted-foreground">{t("sessionTabs.state.empty")}</p> : null}</div></Pane>
          <Pane active={activeTab === "changes"}><div className="grid gap-2">{changes.data?.items.slice(0, 8).map((change) => <div className="flex items-center gap-2 rounded border border-border p-2 text-sm" key={change.path}><GitBranch className="h-4 w-4 text-primary" /><span className="truncate">{change.path}</span></div>)}{!changes.isLoading && !changes.data?.items.length ? <p className="text-xs text-muted-foreground">{t("sessionTabs.changes.clean")}</p> : null}</div></Pane>
        </div>
      </div>
    </aside>
    {collapsed ? <Button className="absolute right-2 top-1/2 h-9 w-9 -translate-y-1/2 px-0" onClick={() => onCollapsedChange(false)} size="icon" title={t("layout.expandInfo")} variant="outline"><PanelRightOpen className="h-4 w-4" /></Button> : null}
  </>;
}
