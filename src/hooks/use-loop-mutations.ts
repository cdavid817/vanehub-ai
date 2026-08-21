import { useMutation, useQueryClient } from "@tanstack/react-query";
import { agentService } from "../services/runtime-agent-client";
import type { LoopDefinition, SaveLoopDefinitionInput } from "../types/loop";
import { loopQueryKeys } from "./loop-query";

export function useStartLoopMutation() {
  const client = useQueryClient();
  return useMutation({
    mutationFn: (definitionId: string) => agentService.startLoop(definitionId),
    onError: (_error, definitionId) => client.invalidateQueries({ queryKey: loopQueryKeys.readiness(definitionId) }),
    onSuccess: ({ run }, definitionId) => {
      client.setQueryData(loopQueryKeys.run(run.id), run);
      void client.invalidateQueries({ queryKey: loopQueryKeys.runs(definitionId) });
      void client.invalidateQueries({ queryKey: loopQueryKeys.readiness(definitionId) });
    },
  });
}

export function useSetLoopEnabledMutation() {
  const client = useQueryClient();
  return useMutation({
    mutationFn: ({ definition, enabled }: { definition: LoopDefinition; enabled: boolean }) => (
      agentService.updateLoopDefinition(definition.id, definitionInput(definition, { enabled }))
    ),
    onSuccess: (definition) => {
      void client.invalidateQueries({ queryKey: loopQueryKeys.definitions });
      void client.invalidateQueries({ queryKey: loopQueryKeys.readiness(definition.id) });
    },
  });
}

export function useDuplicateLoopDefinitionMutation() {
  const client = useQueryClient();
  return useMutation({
    mutationFn: ({ definition, name }: { definition: LoopDefinition; name: string }) => (
      agentService.createLoopDefinition(definitionInput(definition, { enabled: false, name, expectedVersion: null }))
    ),
    onSuccess: () => client.invalidateQueries({ queryKey: loopQueryKeys.definitions }),
  });
}

export function useDeleteLoopDefinitionMutation() {
  const client = useQueryClient();
  return useMutation({
    mutationFn: (definitionId: string) => agentService.deleteLoopDefinition(definitionId),
    onSuccess: (_result, definitionId) => {
      client.removeQueries({ queryKey: loopQueryKeys.readiness(definitionId) });
      void client.invalidateQueries({ queryKey: loopQueryKeys.definitions });
    },
  });
}

function definitionInput(
  definition: LoopDefinition,
  overrides: Partial<SaveLoopDefinitionInput> = {},
): SaveLoopDefinitionInput {
  return {
    name: definition.name,
    enabled: definition.enabled,
    projectPath: definition.projectPath,
    baseBranch: definition.baseBranch,
    goal: definition.goal,
    acceptanceCriteria: [...definition.acceptanceCriteria],
    allowedPaths: [...definition.allowedPaths],
    protectedPaths: [...definition.protectedPaths],
    workerAgentId: definition.workerAgentId,
    verifierAgentId: definition.verifierAgentId,
    verificationCommands: definition.verificationCommands.map((command) => ({ ...command, args: [...command.args] })),
    limits: { ...definition.limits },
    expectedVersion: definition.version,
    ...overrides,
  };
}
