import { Plus, Puzzle, RotateCcw, Upload } from "lucide-react";
import { useMemo, useState } from "react";
import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { useTranslation } from "react-i18next";
import { Button } from "../../components/ui/button";
import { normalizeDisplayPath } from "../../lib/session-path";
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

const emptySkills: Skill[] = [];

export function SkillsPage({ searchTerm }: { searchTerm: string }) {
  const { t } = useTranslation();
  const queryClient = useQueryClient();
  const [scope, setScope] = useState<SkillScope>("global");
  const [workspacePath, setWorkspacePath] = useState("");
  const [category, setCategory] = useState("__all__");
  const [query, setQuery] = useState("");
  const [mountDrafts, setMountDrafts] = useState<Record<string, string>>({});
  const [dialog, setDialog] = useState<SkillDialogState>({ mode: null, skill: null, preview: null });

  const scopeInput = useMemo<SkillScopeInput>(
    () => ({ scope, workspacePath: scope === "workspace" ? workspacePath : null }),
    [scope, workspacePath],
  );
  const scopeReady = scope === "global" || workspacePath.trim().length > 0;

  const overviewQuery = useQuery({
    enabled: scopeReady,
    queryKey: ["skill-overview", scopeInput],
    queryFn: () => agentService.getSkillOverview(scopeInput),
  });

  const invalidate = async () => {
    await queryClient.invalidateQueries({ queryKey: ["skill-overview", scopeInput], exact: true });
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
    mutationFn: ({ skill, agentId, bound }: { skill: Skill; agentId: string; bound: boolean }) =>
      bound
        ? agentService.bindSkillToCliAgent(skill.id, scopeInput, agentId)
        : agentService.unbindSkillFromCliAgent(skill.id, scopeInput, agentId),
    onSuccess: () => void invalidate(),
  });
  const apiBindingMutation = useMutation({
    mutationFn: ({ skill, agentId, bound }: { skill: Skill; agentId: string; bound: boolean }) =>
      bound
        ? agentService.bindSkillToApiAgent(skill.id, scopeInput, agentId)
        : agentService.unbindSkillFromApiAgent(skill.id, scopeInput, agentId),
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
      agentService.updateSkill(skill.id, {
        metadata,
        body,
        expectedContentHash: skill.contentHash,
        ...scopeInput,
      }),
    onSuccess: () => {
      setDialog({ mode: null, skill: null, preview: null });
      void invalidate();
    },
  });
  const editPreviewMutation = useMutation({
    mutationFn: async (skill: Skill) => ({
      skill,
      preview: await agentService.previewSkill(skill.id, scopeInput),
    }),
    onSuccess: ({ skill, preview }) => {
      updateMutation.reset();
      setDialog({ mode: "edit", skill, preview: null, editBody: extractSkillBody(preview.content, skill.metadata.name) });
    },
  });
  const editReloadMutation = useMutation({
    mutationFn: async (skill: Skill) => {
      const overview = await overviewQuery.refetch();
      if (overview.isError) throw overview.error;
      const current = overview.data?.skills.find((candidate) => candidate.id === skill.id);
      if (!current) throw new Error(`Skill no longer exists: ${skill.id}`);
      const preview = await agentService.previewSkill(current.id, scopeInput);
      return { skill: current, body: extractSkillBody(preview.content, current.metadata.name) };
    },
    onSuccess: ({ skill, body }) => {
      updateMutation.reset();
      setDialog({ mode: "edit", skill, preview: null, editBody: body });
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
  const mutationError = [
    mountMutation,
    enabledMutation,
    bindingMutation,
    apiBindingMutation,
    createMutation,
    deleteMutation,
    importMutation,
    restoreMutation,
    previewMutation,
    editPreviewMutation,
    syncMutation,
  ].find((mutation) => mutation.isError)?.error;
  const editError = editReloadMutation.error?.message ?? updateMutation.error?.message ?? null;
  const editConflict = updateMutation.error ? isSkillConflictError(updateMutation.error) : false;

  const skills = overviewQuery.data?.skills ?? emptySkills;
  const stats = overviewQuery.data?.stats ?? { total: 0, enabled: 0, mounted: 0 };
  const categories = useMemo(() => ["__all__", ...Array.from(new Set(skills.map((skill) => skill.metadata.category)))], [skills]);
  const visibleSkills = useMemo(() => {
    const needles = [query, searchTerm]
      .flatMap((value) => value.trim().toLowerCase().split(/\s+/))
      .filter(Boolean);
    return skills.filter((skill) => {
      if (category !== "__all__" && skill.metadata.category !== category) return false;
      if (needles.length === 0) return true;
      const haystack = `${skill.id} ${skill.metadata.name} ${skill.metadata.description} ${skill.metadata.category} ${skill.metadata.triggers.join(" ")} ${skill.source}`
        .toLowerCase();
      return needles.every((needle) => haystack.includes(needle));
    });
  }, [category, query, searchTerm, skills]);

  const cliAgents = overviewQuery.data?.agents.filter((agent) => agent.kind === "cli") ?? [];
  const apiAgents = overviewQuery.data?.agents.filter((agent) => agent.kind === "api") ?? [];
  const apiBindingsBySkillId = overviewQuery.data?.apiAgentBindings ?? {};

  async function browseWorkspace() {
    const selected = await agentService.selectWorkspaceDirectory();
    if (selected) setWorkspacePath(normalizeDisplayPath(selected));
  }

  function toggleAgent(skill: Skill, agentId: string, checked: boolean) {
    bindingMutation.mutate({ skill, agentId, bound: checked });
  }

  function toggleApiAgent(skill: Skill, agentId: string, checked: boolean) {
    apiBindingMutation.mutate({ skill, agentId, bound: checked });
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
      {!scopeReady ? <div className="ucd-panel rounded-lg p-4 text-sm text-muted-foreground">{t("skills.selectWorkspace")}</div> : null}
      {overviewQuery.isLoading ? <div className="ucd-panel rounded-lg p-4 text-sm text-muted-foreground">{t("skills.loading")}</div> : null}
      {overviewQuery.isError ? <div className="ucd-panel rounded-lg p-4 text-sm text-destructive">{overviewQuery.error.message}</div> : null}
      {mutationError ? <div className="ucd-panel rounded-lg p-4 text-sm text-destructive">{mutationError.message}</div> : null}
      {overviewQuery.data ? (
        <>
          <SkillStatsCards stats={stats} />
          <SkillAgentMountPathsPanel
            agents={cliAgents}
            mountPaths={overviewQuery.data.mountPaths}
            drafts={mountDrafts}
            migration={mountMutation.data ?? null}
            savingAgentId={mountMutation.variables?.agentId ?? null}
            onDraftChange={(agentId, value) => setMountDrafts((current) => ({ ...current, [agentId]: value }))}
            onSave={(agentId) => {
              const mountPath = mountDrafts[agentId] ?? overviewQuery.data.mountPaths.find((path) => path.agentId === agentId)?.mountPath ?? "";
              mountMutation.mutate({ agentId, mountPath });
            }}
          />
          <SkillFilterToolbar categories={categories} category={category} query={query} onCategoryChange={setCategory} onQueryChange={setQuery} />
          <SkillDriftBanner drift={overviewQuery.data.drift} syncResult={syncMutation.data ?? null} syncing={syncMutation.isPending} onSync={() => syncMutation.mutate()} />
          <SkillCardList
            agents={cliAgents}
            apiAgents={apiAgents}
            apiBindingsBySkillId={apiBindingsBySkillId}
            busySkillId={enabledMutation.variables?.skill.id ?? bindingMutation.variables?.skill.id ?? apiBindingMutation.variables?.skill.id ?? null}
            skills={visibleSkills}
            onDelete={(skill) => {
              if (globalThis.confirm?.(t("skills.delete")) ?? true) deleteMutation.mutate(skill);
            }}
            onEdit={(skill) => {
              editPreviewMutation.mutate(skill);
            }}
            onPreview={(skill) => previewMutation.mutate(skill)}
            onToggleAgent={toggleAgent}
            onToggleApiAgent={toggleApiAgent}
            onToggleEnabled={(skill, enabled) => enabledMutation.mutate({ skill, enabled })}
          />
          <div className="ucd-panel rounded-lg p-3 text-sm text-muted-foreground">
            {t("skills.showing", { visible: visibleSkills.length, total: skills.length, scope: t(`skills.scope.${scope}`) })}
          </div>
        </>
      ) : null}
      <SkillDialogs
        scope={scope}
        state={dialog}
        restoreCandidates={overviewQuery.data?.restoreCandidates ?? []}
        editConflict={editConflict}
        editError={editError}
        reloadingEdit={editReloadMutation.isPending}
        workspacePath={scope === "workspace" ? workspacePath : null}
        onClose={() => setDialog({ mode: null, skill: null, preview: null })}
        onCreate={(metadata, body, source) => createMutation.mutate({ metadata, body, source })}
        onImport={(sourcePath) => importMutation.mutate(sourcePath)}
        onReloadEdit={(skill) => editReloadMutation.mutate(skill)}
        onRestore={(skillId) => restoreMutation.mutate(skillId)}
        onUpdate={(skill, metadata, body) => updateMutation.mutate({ skill, metadata, body })}
      />
    </div>
  );
}

function extractSkillBody(content: string, name: string) {
  const normalized = content.replaceAll("\r\n", "\n");
  const afterFrontmatter = normalized.startsWith("---\n")
    ? (normalized.split("\n---\n", 2)[1] ?? normalized)
    : normalized;
  const heading = `# ${name}\n`;
  return afterFrontmatter.trimStart().startsWith(heading)
    ? afterFrontmatter.trimStart().slice(heading.length).trim()
    : afterFrontmatter.trim();
}

function isSkillConflictError(error: Error) {
  return error.message.toLowerCase().includes("skill changed since it was loaded");
}
