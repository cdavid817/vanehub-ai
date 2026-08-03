import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { useState } from "react";
import { skillOverviewQueryKey } from "../../../lib/skill-management";
import { agentService } from "../../../services/runtime-agent-client";
import type { Skill, SkillCompatibleAgent, SkillMetadata, SkillScopeInput, SkillSource } from "../../../types/skill";
import { closedSkillDialog, type SkillDialogState } from "./skill-dialogs";

export function useSkillManagement(scopeInput: SkillScopeInput, enabled = true) {
  const queryClient = useQueryClient();
  const [dialog, setDialog] = useState<SkillDialogState>(closedSkillDialog);
  const overviewQuery = useQuery({
    enabled,
    queryKey: skillOverviewQueryKey(scopeInput),
    queryFn: () => agentService.getSkillOverview(scopeInput),
  });
  const invalidate = () => queryClient.invalidateQueries({ queryKey: skillOverviewQueryKey(scopeInput), exact: true });
  const closeDialog = () => setDialog(closedSkillDialog);

  const enabledMutation = useMutation({
    mutationFn: ({ skill, enabled: next }: { skill: Skill; enabled: boolean }) => agentService.setSkillEnabled(skill.id, scopeInput, next),
    onSuccess: invalidate,
  });
  const bindingMutation = useMutation({
    mutationFn: async ({ skill, agent, bound }: { skill: Skill; agent: SkillCompatibleAgent; bound: boolean }) => {
      if (agent.kind === "api") {
        if (bound) await agentService.bindSkillToApiAgent(skill.id, scopeInput, agent.id);
        else await agentService.unbindSkillFromApiAgent(skill.id, scopeInput, agent.id);
        return;
      }
      if (bound) await agentService.bindSkillToCliAgent(skill.id, scopeInput, agent.id);
      else await agentService.unbindSkillFromCliAgent(skill.id, scopeInput, agent.id);
    },
    onSuccess: invalidate,
  });
  const createMutation = useMutation({
    mutationFn: ({ metadata, body, source }: { metadata: SkillMetadata; body: string; source: SkillSource }) => agentService.createSkill({ id: metadata.id, metadata, body, source, enabled: true, boundAgentIds: [], ...scopeInput }),
    onSuccess: () => { closeDialog(); void invalidate(); },
  });
  const updateMutation = useMutation({
    mutationFn: ({ skill, metadata, body }: { skill: Skill; metadata: SkillMetadata; body: string }) => agentService.updateSkill(skill.id, { metadata, body, expectedContentHash: skill.contentHash, ...scopeInput }),
    onSuccess: () => { closeDialog(); void invalidate(); },
  });
  const deleteMutation = useMutation({
    mutationFn: (skill: Skill) => agentService.deleteSkill(skill.id, scopeInput),
    onSuccess: () => { closeDialog(); void invalidate(); },
  });
  const importMutation = useMutation({
    mutationFn: (sourcePath: string) => agentService.importSkill({ sourcePath, enabled: true, boundAgentIds: [], ...scopeInput }),
    onSuccess: () => { closeDialog(); void invalidate(); },
  });
  const restoreMutation = useMutation({
    mutationFn: (skillId: string) => agentService.restoreBuiltinSkill(skillId),
    onSuccess: () => { closeDialog(); void invalidate(); },
  });
  const previewMutation = useMutation({
    mutationFn: (skill: Skill) => agentService.previewSkill(skill.id, scopeInput),
    onSuccess: (preview) => setDialog({ mode: null, skill: null, preview }),
  });
  const editPreviewMutation = useMutation({
    mutationFn: async (skill: Skill) => ({ skill, preview: await agentService.previewSkill(skill.id, scopeInput) }),
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
  const syncMutation = useMutation({ mutationFn: () => agentService.syncSkillDrift(scopeInput), onSuccess: invalidate });

  const rowMutations = [enabledMutation, bindingMutation, previewMutation, editPreviewMutation];
  const failedRowMutation = rowMutations.find((mutation) => mutation.isError);
  const dialogMutation = dialog.mode === "create" ? createMutation
    : dialog.mode === "edit" ? updateMutation
      : dialog.mode === "delete" ? deleteMutation
        : dialog.mode === "import" ? importMutation
          : dialog.mode === "restore" ? restoreMutation
            : null;
  const busyRowMutation = rowMutations.find((mutation) => mutation.isPending);
  return {
    overviewQuery, dialog, setDialog, closeDialog, enabledMutation, bindingMutation, createMutation,
    updateMutation, deleteMutation, importMutation, restoreMutation, previewMutation, editPreviewMutation,
    editReloadMutation, syncMutation,
    dialogOperationError: dialogMutation?.error?.message ?? null,
    dialogPending: dialogMutation?.isPending ?? false,
    rowOperationError: failedRowMutation?.error?.message ?? null,
    rowOperationSkillId: failedRowMutation?.variables && "skill" in failedRowMutation.variables
      ? failedRowMutation.variables.skill.id
      : failedRowMutation?.variables && typeof failedRowMutation.variables === "object" && "id" in failedRowMutation.variables
        ? String(failedRowMutation.variables.id)
        : null,
    busySkillId: busyRowMutation?.variables && "skill" in busyRowMutation.variables
      ? busyRowMutation.variables.skill.id
      : busyRowMutation?.variables && typeof busyRowMutation.variables === "object" && "id" in busyRowMutation.variables
        ? String(busyRowMutation.variables.id)
        : null,
    bindingSkillId: bindingMutation.isPending ? bindingMutation.variables?.skill.id ?? null : null,
  };
}

function extractSkillBody(content: string, name: string) {
  const normalized = content.replaceAll("\r\n", "\n");
  const afterFrontmatter = normalized.startsWith("---\n") ? (normalized.split("\n---\n", 2)[1] ?? normalized) : normalized;
  const heading = `# ${name}\n`;
  return afterFrontmatter.trimStart().startsWith(heading) ? afterFrontmatter.trimStart().slice(heading.length).trim() : afterFrontmatter.trim();
}
