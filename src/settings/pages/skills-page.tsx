import { Plus, Puzzle, RotateCcw, Upload } from "lucide-react";
import { useEffect, useMemo, useState, type ReactNode } from "react";
import { useMutation, useQuery } from "@tanstack/react-query";
import { useTranslation } from "react-i18next";
import { Button } from "../../components/ui/button";
import { effectiveSkillInventory, filterGlobalSkillInventory, isSkillAssigned, isSkillAssignedToAgent, skillIdentity, type SkillInventoryFilters, type SkillInventoryView } from "../../lib/skill-management";
import { useMediaQuery } from "../../hooks/use-media-query";
import { agentService } from "../../services/runtime-agent-client";
import type { SkillCompatibleAgent, SkillOverview, SkillScopeInput } from "../../types/skill";
import type { SkillOverlayTargetInput } from "../../types/skill-overlay";
import type { SettingsPageStatus } from "../settings-page-types";
import { PageHeader, SettingsDisclosure } from "./page-parts";
import { SkillAgentMountPathsPanel } from "./skills/skill-agent-mount-paths-panel";
import { SkillAgentNavigation } from "./skills/skill-agent-navigation";
import { SkillCardList } from "./skills/skill-card-list";
import { SkillDialogs, closedSkillDialog } from "./skills/skill-dialogs";
import { SkillDetailsSurface } from "./skills/skill-details-surface";
import { SkillDriftBanner } from "./skills/skill-drift-banner";
import { SkillFilterToolbar } from "./skills/skill-filter-toolbar";
import { SkillInventorySummary } from "./skills/skill-inventory-summary";
import { useSkillManagement } from "./skills/use-skill-management";

const globalScope: SkillScopeInput = { scope: "global", workspacePath: null };
const defaultFilters: SkillInventoryFilters = { category: "all", query: "", source: "all", status: "all", sort: "name" };

export function SkillsPage({ onStatusChange, searchTerm }: { onStatusChange?: (status: SettingsPageStatus | null) => void; searchTerm: string }) {
  const { t } = useTranslation();
  const manager = useSkillManagement(globalScope);
  const [view, setView] = useState<SkillInventoryView>({ kind: "all" });
  const [filters, setFilters] = useState<SkillInventoryFilters>(defaultFilters);
  const [mountDrafts, setMountDrafts] = useState<Record<string, string>>({});
  const [selectedSkillKey, setSelectedSkillKey] = useState<string | null>(null);
  const [detailsReturnFocus, setDetailsReturnFocus] = useState<HTMLElement | null>(null);
  const wideDetails = useMediaQuery("(min-width: 1280px)");
  const overview = manager.overviewQuery.data;
  const activeAgent = view.kind === "agent" ? overview?.agents.find((agent) => agent.id === view.agentId) ?? null : null;
  const sourceLabels = useMemo(() => ({ builtin: t("skills.source.builtin"), user: t("skills.source.user"), imported: t("skills.source.imported") }), [t]);
  const effectiveFilters = useMemo(() => ({ ...filters, query: `${filters.query} ${searchTerm}`.trim() }), [filters, searchTerm]);
  const visibleSkills = useMemo(() => overview
    ? filterGlobalSkillInventory(overview, view.kind === "agent" ? { kind: "all" } : view, effectiveFilters, sourceLabels)
    : [], [effectiveFilters, overview, sourceLabels, view]);
  const selectedSkill = useMemo(() => visibleSkills.find((skill) => skillIdentity(skill) === selectedSkillKey) ?? null, [selectedSkillKey, visibleSkills]);
  const categories = useMemo(() => ["all", ...new Set(effectiveSkillInventory(overview?.skills ?? []).map((skill) => skill.metadata.category))], [overview?.skills]);
  const counts = useMemo(() => overview ? buildCounts(overview) : {}, [overview]);
  const mountMutation = useMutation({
    mutationFn: ({ agentId, mountPath }: { agentId: string; mountPath: string }) => agentService.updateSkillMountPath(agentId, mountPath),
    onSuccess: () => void manager.overviewQuery.refetch(),
  });
  const detailQuery = useQuery({
    enabled: Boolean(selectedSkill),
    queryKey: ["skill-runtime-details", selectedSkill?.id ?? "", selectedSkill?.contentHash ?? ""],
    queryFn: () => agentService.loadSkill({ idOrAlias: selectedSkill!.id, workspacePath: selectedSkill!.workspacePath }),
    staleTime: 30_000,
  });
  const overlayTarget = useMemo(() => selectedSkill ? skillOverlayTarget(selectedSkill.id, selectedSkill.workspacePath) : null, [selectedSkill]);
  const overlayQuery = useQuery({
    enabled: Boolean(overlayTarget),
    queryKey: ["skill-overlay-details", overlayTarget?.skillId ?? "", selectedSkill?.contentHash ?? "", overlayTarget?.workspacePath ?? ""],
    queryFn: () => agentService.getSkillOverlayDetail(overlayTarget!),
    staleTime: 30_000,
  });
  useEffect(() => {
    if (selectedSkillKey && !selectedSkill) {
      setSelectedSkillKey(null);
      setDetailsReturnFocus(null);
    }
  }, [selectedSkill, selectedSkillKey]);
  const activeMigration = activeAgent && mountMutation.data?.agentId === activeAgent.id ? mountMutation.data : null;
  const mountError = activeAgent && mountMutation.variables?.agentId === activeAgent.id ? mountMutation.error?.message ?? null : null;
  const filtered = Boolean(effectiveFilters.query || filters.category !== "all" || filters.source !== "all" || filters.status !== "all");
  const editError = manager.editReloadMutation.error?.message ?? manager.updateMutation.error?.message ?? null;

  // Task 12.16: four already-rendered error surfaces above, combined as one boolean -- all the
  // same "error" kind, so which one fired doesn't change what the nav entry shows.
  useEffect(() => {
    const hasError = Boolean(manager.overviewQuery.isError || manager.rowOperationError || mountError || manager.dialogOperationError);
    onStatusChange?.(hasError ? { kind: "error", labelKey: "skills.status.error" } : null);
    return () => onStatusChange?.(null);
  }, [manager.dialogOperationError, manager.overviewQuery.isError, manager.rowOperationError, mountError, onStatusChange]);

  return <div className="space-y-4">
    <PageHeader actions={<><Button onClick={() => manager.setDialog({ mode: "restore", skill: null, preview: null })} variant="outline"><RotateCcw />{t("skills.restoreBuiltIn")}</Button><Button onClick={() => manager.setDialog({ mode: "import", skill: null, preview: null })} variant="outline"><Upload />{t("skills.importSkill")}</Button><Button onClick={() => manager.setDialog({ mode: "create", skill: null, preview: null })}><Plus />{t("skills.createSkill")}</Button></>} description={t("skills.descriptionGlobal")} icon={Puzzle} title={t("skills.title")} />
    {manager.overviewQuery.isLoading ? <Status>{t("skills.loading")}</Status> : null}
    {manager.overviewQuery.isError ? <Status danger><div className="flex flex-wrap items-center justify-between gap-3"><span>{manager.overviewQuery.error.message}</span><Button onClick={() => void manager.overviewQuery.refetch()} size="sm" variant="outline">{t("featureLoad.retry")}</Button></div></Status> : null}
    {overview ? <div className="grid gap-4 lg:grid-cols-[220px_minmax(0,1fr)]">
      <SkillAgentNavigation agents={overview.agents} counts={counts} onSelect={setView} selected={view} />
      <div className="min-w-0 space-y-4">
        <section className="ucd-panel rounded-lg p-4"><div><h3 className="text-base font-semibold">{viewTitle(view, activeAgent, t)}</h3><p className="mt-1 text-xs text-muted-foreground">{activeAgent ? t(activeAgent.kind === "api" ? "skills.assignment.apiExplanation" : "skills.assignment.cliExplanation") : t("skills.enablementExplanation")}</p></div><div className="mt-3 border-t border-border pt-3"><SkillInventorySummary overview={overview} view={view} /></div></section>
        <SkillFilterToolbar categories={categories} filters={filters} onChange={setFilters} resultCount={visibleSkills.length} />
        <SkillDriftBanner drift={overview.drift} onDismiss={() => manager.syncMutation.reset()} onSync={() => manager.syncMutation.mutate()} syncError={manager.syncMutation.error?.message} syncResult={manager.syncMutation.data ?? null} syncing={manager.syncMutation.isPending} />
        <div className={selectedSkill && wideDetails ? "grid min-w-0 items-start gap-4 xl:grid-cols-[minmax(0,1fr)_22rem]" : "min-w-0"}>
          <SkillCardList activeAgent={activeAgent} apiBindingsBySkillId={overview.apiAgentBindings} bindingSkillId={manager.bindingSkillId} busySkillId={manager.busySkillId} filtered={filtered} onDelete={(skill) => manager.setDialog({ mode: "delete", skill, preview: null })} onDetails={(skill, trigger) => { setSelectedSkillKey(skillIdentity(skill)); setDetailsReturnFocus(trigger); }} onEdit={(skill) => manager.editPreviewMutation.mutate(skill)} onPreview={(skill) => manager.previewMutation.mutate(skill)} onToggleAgent={(skill, assigned) => activeAgent && manager.bindingMutation.mutate({ skill, agent: activeAgent, bound: assigned })} onToggleEnabled={(skill, enabled) => manager.enabledMutation.mutate({ skill, enabled })} operationError={manager.rowOperationError} operationSkillId={manager.rowOperationSkillId} selectedSkillKey={selectedSkillKey} skills={visibleSkills} />
          {selectedSkill && overlayTarget ? <SkillDetailsSurface loadOutcome={detailQuery.data} loading={detailQuery.isLoading || detailQuery.isFetching} onClose={() => { setSelectedSkillKey(null); setDetailsReturnFocus(null); }} onOverlayCommitted={() => void overlayQuery.refetch()} onRetryOverlay={() => overlayQuery.refetch()} overlayDetail={overlayQuery.data} overlayError={overlayQuery.error?.message} overlayLoading={overlayQuery.isLoading || overlayQuery.isFetching} overlayTarget={overlayTarget} returnFocus={detailsReturnFocus} skill={selectedSkill} wide={wideDetails} /> : null}
        </div>
        {activeAgent?.kind === "cli" && (mountError || activeMigration?.failed.length) ? <div className="rounded-lg border border-destructive/40 bg-destructive/10 p-3 text-sm text-destructive" role="alert">{mountError ?? t("skills.mountPaths.failed", { count: activeMigration?.failed.length ?? 0 })}</div> : null}
        {activeAgent?.kind === "cli" ? <SettingsDisclosure description={t("skills.mountPaths.description")} title={t("skills.mountPaths.title")}><SkillAgentMountPathsPanel agents={[activeAgent]} drafts={mountDrafts} error={mountError} migration={activeMigration} mountPaths={overview.mountPaths} onDraftChange={(agentId, value) => setMountDrafts((current) => ({ ...current, [agentId]: value }))} onSave={(agentId) => mountMutation.mutate({ agentId, mountPath: mountDrafts[agentId] ?? overview.mountPaths.find((path) => path.agentId === agentId)?.mountPath ?? "" })} savingAgentId={mountMutation.isPending ? mountMutation.variables?.agentId ?? null : null} /></SettingsDisclosure> : null}
      </div>
    </div> : null}
    <SkillDialogs editConflict={Boolean(manager.updateMutation.error?.message.toLowerCase().includes("skill changed since it was loaded"))} editError={editError} onClose={() => { manager.setDialog(closedSkillDialog); manager.deleteMutation.reset(); }} onCreate={(metadata, body, source) => manager.createMutation.mutate({ metadata, body, source })} onDelete={(skill) => manager.deleteMutation.mutate(skill)} onImport={(sourcePath) => manager.importMutation.mutate(sourcePath)} onReloadEdit={(skill) => manager.editReloadMutation.mutate(skill)} onRestore={(skillId) => manager.restoreMutation.mutate(skillId)} onUpdate={(skill, metadata, body) => manager.updateMutation.mutate({ skill, metadata, body })} operationError={manager.dialogOperationError} operationPending={manager.dialogPending} reloadingEdit={manager.editReloadMutation.isPending} restoreCandidates={overview?.restoreCandidates} scope="global" state={manager.dialog} workspacePath={null} />
  </div>;
}

function buildCounts(overview: SkillOverview) {
  const skills = effectiveSkillInventory(overview.skills);
  const counts: Record<string, number> = { all: skills.length, unassigned: skills.filter((skill) => !isSkillAssigned(skill, overview)).length };
  for (const agent of overview.agents) counts[`agent:${agent.id}`] = skills.filter((skill) => isSkillAssignedToAgent(skill, agent, overview.apiAgentBindings)).length;
  return counts;
}

function skillOverlayTarget(skillId: string, workspacePath: string | null): SkillOverlayTargetInput {
  return workspacePath
    ? { skillId, scope: "project", workspacePath }
    : { skillId, scope: "user", workspacePath: null };
}

function viewTitle(view: SkillInventoryView, agent: SkillCompatibleAgent | null, t: (key: string, options?: Record<string, unknown>) => string) {
  if (view.kind === "agent") return agent?.displayName ?? view.agentId;
  return t(view.kind === "unassigned" ? "skills.navigation.unassigned" : "skills.navigation.all");
}

function Status({ children, danger }: { children: ReactNode; danger?: boolean }) {
  return <div className={`ucd-panel rounded-lg p-4 text-sm ${danger ? "text-destructive" : "text-muted-foreground"}`}>{children}</div>;
}
