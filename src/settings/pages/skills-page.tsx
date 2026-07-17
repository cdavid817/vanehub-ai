import { Plus, Puzzle, RotateCcw, Upload } from "lucide-react";
import { useMemo, useState } from "react";
import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { useTranslation } from "react-i18next";
import { Button } from "../../components/ui/button";
import { agentService } from "../../services/runtime-agent-client";
import type { Skill, SkillMetadata, SkillScope, SkillScopeInput, SkillSource } from "../../types/skill";
import { PageHeader } from "./page-parts";
import { SkillAgentMountPathsPanel } from "./skills/skill-agent-mount-paths-panel";
import { SkillCardList } from "./skills/skill-card-list";
import { SkillDialogs, type SkillDialogState } from "./skills/skill-dialogs";
import { SkillDriftBanner } from "./skills/skill-drift-banner";
import { SkillFilterToolbar } from "./skills/skill-filter-toolbar";
import { SkillScopeTabs } from "./skills/skill-scope-tabs";
import { SkillStatsCards } from "./skills/skill-stats-cards";

export function SkillsPage({ searchTerm }: { searchTerm: string }) {
  const { t } = useTranslation();
  const queryClient = useQueryClient();
  const [scope, setScope] = useState<SkillScope>("global");
  const [workspacePath, setWorkspacePath] = useState("");
  const [category, setCategory] = useState("__all__");
  const [query, setQuery] = useState(searchTerm);
  const [mountDrafts, setMountDrafts] = useState<Record<string, string>>({});
  const [dialog, setDialog] = useState<SkillDialogState>({ mode: null, skill: null, preview: null });

  const scopeInput = useMemo<SkillScopeInput>(
    () => ({ scope, workspacePath: scope === "workspace" ? workspacePath : null }),
    [scope, workspacePath],
  );
  const scopeReady = scope === "global" || workspacePath.trim().length > 0;

  const agentsQuery = useQuery({ queryKey: ["agents", "skills"], queryFn: () => agentService.listAgents() });
  const mountPathsQuery = useQuery({ queryKey: ["skill-mount-paths"], queryFn: () => agentService.listSkillMountPaths() });
  const skillsQuery = useQuery({
    enabled: scopeReady,
    queryKey: ["skills", scopeInput],
    queryFn: () => agentService.listSkills(scopeInput),
  });
  const driftQuery = useQuery({
    enabled: scopeReady,
    queryKey: ["skill-drift", scopeInput],
    queryFn: () => agentService.detectSkillDrift(scopeInput),
  });

  const invalidate = async () => {
    await Promise.all([
      queryClient.invalidateQueries({ queryKey: ["skills"] }),
      queryClient.invalidateQueries({ queryKey: ["skill-drift"] }),
      queryClient.invalidateQueries({ queryKey: ["skill-mount-paths"] }),
    ]);
  };

  const mountMutation = useMutation({
    mutationFn: ({ agentId, mountPath }: { agentId: string; mountPath: string }) =>
      agentService.updateSkillMountPath(agentId, mountPath),
    onSuccess: () => void invalidate(),
  });
  const enabledMutation = useMutation({
    mutationFn: ({ skill, enabled }: { skill: Skill; enabled: boolean }) =>
      agentService.setSkillEnabled(skill.id, scopeInput, enabled),
    onSuccess: () => void invalidate(),
  });
  const bindingMutation = useMutation({
    mutationFn: ({ skill, agentIds }: { skill: Skill; agentIds: string[] }) =>
      agentService.setSkillAgentBindings(skill.id, scopeInput, agentIds),
    onSuccess: () => void invalidate(),
  });
  const createMutation = useMutation({
    mutationFn: ({ metadata, body, source }: { metadata: SkillMetadata; body: string; source: SkillSource }) =>
      agentService.createSkill({
        id: metadata.id,
        metadata,
        body,
        source,
        enabled: true,
        boundAgentIds: [],
        ...scopeInput,
      }),
    onSuccess: () => {
      setDialog({ mode: null, skill: null, preview: null });
      void invalidate();
    },
  });
  const updateMutation = useMutation({
    mutationFn: ({ skill, metadata, body }: { skill: Skill; metadata: SkillMetadata; body: string }) =>
      agentService.updateSkill(skill.id, { metadata, body, enabled: skill.enabled, boundAgentIds: skill.boundAgentIds, ...scopeInput }),
    onSuccess: () => {
      setDialog({ mode: null, skill: null, preview: null });
      void invalidate();
    },
  });
  const deleteMutation = useMutation({
    mutationFn: (skill: Skill) => agentService.deleteSkill(skill.id, scopeInput),
    onSuccess: () => void invalidate(),
  });
  const importMutation = useMutation({
    mutationFn: (sourcePath: string) => agentService.importSkill({ sourcePath, enabled: true, boundAgentIds: [], ...scopeInput }),
    onSuccess: () => {
      setDialog({ mode: null, skill: null, preview: null });
      void invalidate();
    },
  });
  const restoreMutation = useMutation({
    mutationFn: (skillId: string) => agentService.restoreBuiltinSkill(skillId),
    onSuccess: () => {
      setDialog({ mode: null, skill: null, preview: null });
      void invalidate();
    },
  });
  const previewMutation = useMutation({
    mutationFn: (skill: Skill) => agentService.previewSkill(skill.id, scopeInput),
    onSuccess: (preview) => setDialog({ mode: null, skill: null, preview }),
  });
  const syncMutation = useMutation({
    mutationFn: () => agentService.syncSkillDrift(scopeInput),
    onSuccess: () => void invalidate(),
  });

  const skills = skillsQuery.data?.skills ?? [];
  const stats = skillsQuery.data?.stats ?? { total: 0, enabled: 0, mounted: 0 };
  const categories = useMemo(() => ["__all__", ...Array.from(new Set(skills.map((skill) => skill.metadata.category)))], [skills]);
  const visibleSkills = useMemo(() => {
    const needle = `${query} ${searchTerm}`.trim().toLowerCase();
    return skills.filter((skill) => {
      if (category !== "__all__" && skill.metadata.category !== category) return false;
      if (!needle) return true;
      return `${skill.id} ${skill.metadata.name} ${skill.metadata.description} ${skill.metadata.category} ${skill.metadata.triggers.join(" ")} ${skill.source}`
        .toLowerCase()
        .includes(needle);
    });
  }, [category, query, searchTerm, skills]);

  async function browseWorkspace() {
    const selected = await agentService.selectWorkspaceDirectory();
    if (selected) setWorkspacePath(selected);
  }

  function toggleAgent(skill: Skill, agentId: string, checked: boolean) {
    const agentIds = checked
      ? Array.from(new Set([...skill.boundAgentIds, agentId]))
      : skill.boundAgentIds.filter((id) => id !== agentId);
    bindingMutation.mutate({ skill, agentIds });
  }

  return (
    <div className="space-y-4">
      <PageHeader
        actions={
          <>
            <Button onClick={() => setDialog({ mode: "restore", skill: null, preview: null })} variant="outline">
              <RotateCcw className="h-4 w-4" aria-hidden="true" />
              {t("skills.restoreBuiltIn")}
            </Button>
            <Button onClick={() => setDialog({ mode: "import", skill: null, preview: null })} variant="outline">
              <Upload className="h-4 w-4" aria-hidden="true" />
              {t("skills.importSkill")}
            </Button>
            <Button onClick={() => setDialog({ mode: "create", skill: null, preview: null })}>
              <Plus className="h-4 w-4" aria-hidden="true" />
              {t("skills.createSkill")}
            </Button>
          </>
        }
        description={t("skills.description")}
        icon={Puzzle}
        title={t("skills.title")}
      />

      <SkillScopeTabs scope={scope} workspacePath={workspacePath} onScopeChange={setScope} onWorkspacePathChange={setWorkspacePath} onBrowse={() => void browseWorkspace()} />
      <SkillStatsCards stats={stats} />
      <SkillAgentMountPathsPanel
        agents={agentsQuery.data ?? []}
        mountPaths={mountPathsQuery.data ?? []}
        drafts={mountDrafts}
        migration={mountMutation.data ?? null}
        savingAgentId={mountMutation.variables?.agentId ?? null}
        onDraftChange={(agentId, value) => setMountDrafts((current) => ({ ...current, [agentId]: value }))}
        onSave={(agentId) => {
          const mountPath = mountDrafts[agentId] ?? mountPathsQuery.data?.find((path) => path.agentId === agentId)?.mountPath ?? "";
          mountMutation.mutate({ agentId, mountPath });
        }}
      />
      <SkillFilterToolbar categories={categories} category={category} query={query} onCategoryChange={setCategory} onQueryChange={setQuery} />
      <SkillDriftBanner drift={driftQuery.data ?? null} syncResult={syncMutation.data ?? null} syncing={syncMutation.isPending} onSync={() => syncMutation.mutate()} />
      {!scopeReady ? <div className="ucd-panel rounded-lg p-4 text-sm text-muted-foreground">{t("skills.selectWorkspace")}</div> : null}
      {skillsQuery.isLoading ? <div className="ucd-panel rounded-lg p-4 text-sm text-muted-foreground">{t("skills.loading")}</div> : null}
      <SkillCardList
        agents={agentsQuery.data ?? []}
        busySkillId={enabledMutation.variables?.skill.id ?? bindingMutation.variables?.skill.id ?? null}
        skills={visibleSkills}
        onDelete={(skill) => deleteMutation.mutate(skill)}
        onEdit={(skill) => setDialog({ mode: "edit", skill, preview: null })}
        onPreview={(skill) => previewMutation.mutate(skill)}
        onToggleAgent={toggleAgent}
        onToggleEnabled={(skill, enabled) => enabledMutation.mutate({ skill, enabled })}
      />
      <div className="ucd-panel rounded-lg p-3 text-sm text-muted-foreground">
        {t("skills.showing", { visible: visibleSkills.length, total: skills.length, scope: t(`skills.scope.${scope}`) })}
      </div>
      <SkillDialogs
        scope={scope}
        state={dialog}
        workspacePath={scope === "workspace" ? workspacePath : null}
        onClose={() => setDialog({ mode: null, skill: null, preview: null })}
        onCreate={(metadata, body, source) => createMutation.mutate({ metadata, body, source })}
        onImport={(sourcePath) => importMutation.mutate(sourcePath)}
        onRestore={(skillId) => restoreMutation.mutate(skillId)}
        onUpdate={(skill, metadata, body) => updateMutation.mutate({ skill, metadata, body })}
      />
    </div>
  );
}
