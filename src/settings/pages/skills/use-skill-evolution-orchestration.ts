import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { useCallback, useMemo } from "react";
import type {
  EvolutionPolicyUpdate,
  EvolutionRunSummary,
  SkillEvolutionOrchestrationService,
} from "../../../services/skill-evolution-orchestration-service";

export function useSkillEvolutionOrchestration(
  workspaceId: string,
  service: SkillEvolutionOrchestrationService,
) {
  const queryClient = useQueryClient();
  const enabled = Boolean(workspaceId);
  const key = useMemo(() => ["skill-evolution-orchestration", workspaceId] as const, [workspaceId]);
  const overview = useQuery({
    enabled,
    queryKey: [...key, "overview"],
    queryFn: () => service.getEvolutionSchedulerOverview(workspaceId),
    refetchInterval: enabled ? 5_000 : false,
  });
  const policy = useQuery({
    enabled,
    queryKey: [...key, "policy"],
    queryFn: () => service.getEvolutionPolicy(workspaceId),
  });
  const runs = useQuery({
    enabled,
    queryKey: [...key, "runs"],
    queryFn: () => service.listEvolutionRuns({ workspaceId, limit: 50 }),
    refetchInterval: (query) => query.state.data?.items.some(isActiveRun) ? 2_000 : false,
  });
  const eligibility = useQuery({
    enabled,
    queryKey: [...key, "eligibility"],
    queryFn: () => service.listEvolutionEligibility({ workspaceId, limit: 50 }),
  });
  const applications = useQuery({
    enabled,
    queryKey: [...key, "applications"],
    queryFn: () => service.listEvolutionApplications({ workspaceId, limit: 50 }),
  });
  const probations = useQuery({
    enabled,
    queryKey: [...key, "probations"],
    queryFn: () => service.listEvolutionProbations({ workspaceId, limit: 50 }),
  });
  const breakers = useQuery({
    enabled,
    queryKey: [...key, "breakers"],
    queryFn: () => service.listEvolutionBreakers({ workspaceId, limit: 50 }),
  });
  const refresh = useCallback(async () => {
    await queryClient.invalidateQueries({ queryKey: key });
  }, [key, queryClient]);
  const updatePolicy = useMutation({
    mutationFn: (input: EvolutionPolicyUpdate) => service.updateEvolutionPolicy(input),
    onSuccess: refresh,
  });
  const requestRun = useMutation({
    mutationFn: () => service.requestEvolutionRun(workspaceId),
    onSuccess: refresh,
  });
  const cancelRun = useMutation({
    mutationFn: (run: EvolutionRunSummary) => service.cancelEvolutionRun(run.runId, run.revision),
    onSuccess: refresh,
  });
  const acknowledgeBreaker = useMutation({
    mutationFn: ({ breakerId, revision }: { breakerId: string; revision: number }) =>
      service.acknowledgeEvolutionBreaker(breakerId, revision),
    onSuccess: refresh,
  });
  return {
    acknowledgeBreaker,
    applications,
    breakers,
    cancelRun,
    eligibility,
    overview,
    policy,
    probations,
    refresh,
    requestRun,
    runs,
    updatePolicy,
  };
}

function isActiveRun(run: EvolutionRunSummary) {
  return ["requested", "waiting_idle", "running", "partial", "cancel_requested", "recovered"]
    .includes(run.status);
}
