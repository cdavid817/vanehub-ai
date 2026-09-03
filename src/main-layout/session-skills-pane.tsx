import { useMemo, useState, type ReactNode } from "react";
import { useQuery } from "@tanstack/react-query";
import { Eye, Pencil, Plus, Settings, Trash2, Upload } from "lucide-react";
import { useTranslation } from "react-i18next";
import { Badge } from "../components/ui/badge";
import { Button } from "../components/ui/button";
import { cn } from "../lib/utils";
import { deriveSessionSkillGroups, getSkillBindingState, resolveSessionSkillWorkspace, skillIdentity, skillOverviewQueryKey } from "../lib/skill-management";
import { normalizeDisplayPath } from "../lib/session-path";
import { agentService } from "../services/runtime-agent-client";
import { SkillDialogs, closedSkillDialog } from "../settings/pages/skills/skill-dialogs";
import { SkillDriftBanner } from "../settings/pages/skills/skill-drift-banner";
import { useSkillManagement } from "../settings/pages/skills/use-skill-management";
import type { Session } from "../types/agent";
import type { Skill, SkillCompatibleAgent, SkillScopeInput } from "../types/skill";
import { useTabList } from "../ui/runtime-panel/use-tab-list";

type SkillSubview = "effective" | "global" | "project";
const skillSubviews: SkillSubview[] = ["effective", "global", "project"];
const globalScope: SkillScopeInput = { scope: "global", workspacePath: null };

export function SessionSkillsPane({ active = true, activeSession, onOpenSkillSettings }: {
  /**
   * Whether this pane is the one on screen.
   *
   * Mounted either way — these panes hold local form state, and a reader who typed something,
   * checked another tab, and came back must find it still there. What stops is the reading: a
   * hidden pane polling its own service costs a request per pane per session open, for answers
   * nobody is looking at.
   *
   * Mutations are unaffected. React Query runs one to completion regardless of this flag, so a
   * write that was in flight when the reader switched away still finishes and still invalidates.
   */
  active?: boolean;
  activeSession: Session | null;
  onOpenSkillSettings?: () => void;
}) {
  const { t } = useTranslation();
  const [activeView, setActiveView] = useState<SkillSubview>("effective");
  const workspacePath = resolveSessionSkillWorkspace(activeSession);
  const projectScope = useMemo<SkillScopeInput>(() => ({ scope: "workspace", workspacePath }), [workspacePath]);
  const project = useSkillManagement(projectScope, active && Boolean(activeSession && workspacePath));
  const global = useQuery({ enabled: active && Boolean(activeSession), queryKey: skillOverviewQueryKey(globalScope), queryFn: () => agentService.getSkillOverview(globalScope) });
  const agent = useMemo<SkillCompatibleAgent | null>(() => {
    if (!activeSession) return null;
    return global.data?.agents.find((candidate) => candidate.id === activeSession.agentId)
      ?? project.overviewQuery.data?.agents.find((candidate) => candidate.id === activeSession.agentId)
      ?? { id: activeSession.agentId, displayName: activeSession.agentId, kind: activeSession.interactionMode === "api" ? "api" : "cli" };
  }, [activeSession, global.data?.agents, project.overviewQuery.data?.agents]);
  const groups = useMemo(() => deriveSessionSkillGroups({ agent, globalOverview: global.data, projectOverview: project.overviewQuery.data }), [agent, global.data, project.overviewQuery.data]);
  const loading = global.isLoading || (Boolean(workspacePath) && project.overviewQuery.isLoading);
  const error = global.error?.message ?? project.overviewQuery.error?.message ?? null;
  const editError = project.editReloadMutation.error?.message ?? project.updateMutation.error?.message ?? null;
  const viewCounts: Record<SkillSubview, number> = { effective: groups.effective.length, global: groups.global.length, project: groups.project.length };
  const skillTabs = useTabList(skillSubviews.map((view) => ({ id: view })), activeView, (id) => setActiveView(id as SkillSubview));

  return <div className="grid gap-3">
    <div className="ucd-segmented grid grid-cols-3 gap-1 rounded-md p-1" onKeyDown={skillTabs.handleKeyDown} role="tablist">
      {skillSubviews.map((view) => <button aria-selected={activeView === view} className={cn("h-8 truncate rounded-md px-1 text-xs", activeView === view ? "bg-background font-semibold text-primary shadow-xs" : "text-muted-foreground hover:bg-muted")} key={view} onClick={() => setActiveView(view)} ref={skillTabs.registerTabRef(view)} role="tab" tabIndex={activeView === view ? 0 : -1} type="button"><span>{t(`layout.info.skills.views.${view}`)}</span><span aria-hidden="true" className="ml-1 tabular-nums text-muted-foreground">{t("layout.info.skills.viewCount", { count: viewCounts[view] })}</span></button>)}
    </div>
    {loading ? <Empty>{t("layout.info.loading")}</Empty> : null}
    {error ? <div className="flex flex-wrap items-center justify-between gap-2 rounded border border-destructive/40 p-3 text-xs text-destructive" role="alert"><span>{error}</span><Button onClick={() => { void global.refetch(); if (workspacePath) void project.overviewQuery.refetch(); }} size="sm" variant="outline">{t("featureLoad.retry")}</Button></div> : null}
    <div className={activeView === "effective" ? "grid gap-2" : "hidden"}>
      <p className="text-xs leading-5 text-muted-foreground">{t("layout.info.skills.effectiveDescription")}</p>
      <InfoSkillRows agent={agent} globalApiBindings={global.data?.apiAgentBindings} projectApiBindings={project.overviewQuery.data?.apiAgentBindings} skills={groups.effective} />
      {!loading && groups.effective.length === 0 ? <Empty>{t("layout.info.skills.noAvailable")}</Empty> : null}
    </div>
    <div className={activeView === "global" ? "grid gap-2" : "hidden"}>
      <div className="flex items-center justify-between gap-2"><p className="text-xs leading-5 text-muted-foreground">{t("layout.info.skills.globalDescription")}</p>{onOpenSkillSettings ? <Button onClick={onOpenSkillSettings} size="sm" variant="outline"><Settings />{t("layout.info.skills.manageGlobal")}</Button> : null}</div>
      <InfoSkillRows agent={agent} globalApiBindings={global.data?.apiAgentBindings} skills={groups.global} />
      {!global.isLoading && groups.global.length === 0 ? <Empty>{t("layout.info.skills.noGlobal")}</Empty> : null}
    </div>
    <div className={activeView === "project" ? "grid gap-3" : "hidden"}>
      {workspacePath ? <>
        <div className="rounded border border-border bg-background p-2"><p className="text-[11px] text-muted-foreground">{t("layout.info.skills.projectPath")}</p><p className="mt-1 break-all font-mono text-xs">{normalizeDisplayPath(workspacePath)}</p></div>
        <div className="flex flex-wrap gap-2"><Button onClick={() => project.setDialog({ mode: "create", skill: null, preview: null })} size="sm"><Plus />{t("skills.createSkill")}</Button><Button onClick={() => project.setDialog({ mode: "import", skill: null, preview: null })} size="sm" variant="outline"><Upload />{t("skills.importSkill")}</Button></div>
        {project.overviewQuery.data ? <>
          <SkillDriftBanner drift={project.overviewQuery.data.drift} onDismiss={() => project.syncMutation.reset()} onSync={() => project.syncMutation.mutate()} syncError={project.syncMutation.error?.message} syncResult={project.syncMutation.data ?? null} syncing={project.syncMutation.isPending} />
          <ProjectSkillRows agent={agent} apiBindings={project.overviewQuery.data.apiAgentBindings} busySkillId={project.busySkillId} onDelete={(skill) => project.setDialog({ mode: "delete", skill, preview: null })} onEdit={(skill) => project.editPreviewMutation.mutate(skill)} onPreview={(skill) => project.previewMutation.mutate(skill)} onToggleAgent={(skill, bound) => agent && project.bindingMutation.mutate({ skill, agent, bound })} onToggleEnabled={(skill, enabled) => project.enabledMutation.mutate({ skill, enabled })} operationError={project.rowOperationError} operationSkillId={project.rowOperationSkillId} skills={groups.project} />
          {groups.project.length === 0 ? <Empty>{t("layout.info.skills.noProject")}</Empty> : null}
        </> : null}
      </> : <Empty>{t("layout.info.skills.noWorkspace")}</Empty>}
    </div>
    <SkillDialogs editConflict={Boolean(project.updateMutation.error?.message.toLowerCase().includes("skill changed since it was loaded"))} editError={editError} onClose={() => project.setDialog(closedSkillDialog)} onCreate={(metadata, body, source) => project.createMutation.mutate({ metadata, body, source })} onDelete={(skill) => project.deleteMutation.mutate(skill)} onImport={(sourcePath) => project.importMutation.mutate(sourcePath)} onReloadEdit={(skill) => project.editReloadMutation.mutate(skill)} onUpdate={(skill, metadata, body) => project.updateMutation.mutate({ skill, metadata, body })} operationError={project.dialogOperationError} operationPending={project.dialogPending} reloadingEdit={project.editReloadMutation.isPending} scope="workspace" state={project.dialog} workspacePath={workspacePath} />
  </div>;
}

function InfoSkillRows({ skills, agent, globalApiBindings = {}, projectApiBindings = {} }: { skills: Skill[]; agent: SkillCompatibleAgent | null; globalApiBindings?: Record<string, string[]>; projectApiBindings?: Record<string, string[]> }) {
  const { t } = useTranslation();
  return <div className="grid gap-2">{skills.map((skill) => <article className="rounded border border-border bg-background p-2 text-sm" key={skillIdentity(skill)}><div className="flex min-w-0 flex-wrap items-center gap-1.5"><h4 className="min-w-0 flex-1 truncate font-medium">{skill.metadata.name}</h4><Badge tone="muted">{t(`skills.scope.${skill.scope}`)}</Badge>{agent ? <Badge tone={skill.enabled ? "default" : "muted"}>{t(`skills.binding.${getSkillBindingState(skill, agent, skill.scope === "global" ? globalApiBindings : projectApiBindings)}`)}</Badge> : null}</div><p className="mt-1 line-clamp-2 text-xs text-muted-foreground">{skill.metadata.description}</p></article>)}</div>;
}

function ProjectSkillRows({ skills, agent, apiBindings, busySkillId, operationError, operationSkillId, onToggleEnabled, onToggleAgent, onPreview, onEdit, onDelete }: { skills: Skill[]; agent: SkillCompatibleAgent | null; apiBindings: Record<string, string[]>; busySkillId: string | null; operationError: string | null; operationSkillId: string | null; onToggleEnabled: (skill: Skill, enabled: boolean) => void; onToggleAgent: (skill: Skill, bound: boolean) => void; onPreview: (skill: Skill) => void; onEdit: (skill: Skill) => void; onDelete: (skill: Skill) => void }) {
  const { t } = useTranslation();
  return <div className="grid gap-2">{skills.map((skill) => {
    const state = agent ? getSkillBindingState(skill, agent, apiBindings) : "available";
    const assigned = state !== "available";
    return <article className="rounded border border-border bg-background p-2" key={skillIdentity(skill)}><div className="flex flex-wrap items-center gap-1.5"><h4 className="min-w-0 flex-1 truncate text-sm font-medium">{skill.metadata.name}</h4><Badge tone={skill.enabled ? "success" : "muted"}>{t(skill.enabled ? "skills.enabled" : "basic.disabled")}</Badge>{agent ? <Badge tone={assigned ? "default" : "muted"}>{t(`skills.binding.${state}`)}</Badge> : null}</div><p className="mt-1 line-clamp-2 text-xs text-muted-foreground">{skill.metadata.description}</p><div className="mt-2 flex flex-wrap items-center gap-1"><label className="mr-auto flex items-center gap-1.5 text-xs"><input aria-label={t("skills.enabled")} checked={skill.enabled} disabled={busySkillId === skill.id} onChange={(event) => onToggleEnabled(skill, event.target.checked)} type="checkbox" />{t("skills.enabled")}</label>{agent ? <Button disabled={busySkillId === skill.id} onClick={() => onToggleAgent(skill, !assigned)} size="sm" variant="outline">{t(assigned ? "skills.assignment.remove" : "skills.assignment.add")}</Button> : null}<Button aria-label={t("skills.preview")} disabled={busySkillId === skill.id} onClick={() => onPreview(skill)} size="icon" variant="ghost"><Eye /></Button><Button aria-label={t("skills.edit")} disabled={busySkillId === skill.id} onClick={() => onEdit(skill)} size="icon" variant="ghost"><Pencil /></Button><Button aria-label={t("skills.delete")} onClick={() => onDelete(skill)} size="icon" variant="ghost"><Trash2 /></Button>{operationSkillId === skill.id && operationError ? <p className="basis-full text-xs text-destructive" role="alert">{operationError}</p> : null}</div></article>;
  })}</div>;
}

function Empty({ children }: { children: ReactNode }) {
  return <p className="rounded border border-border bg-background p-3 text-xs text-muted-foreground">{children}</p>;
}
