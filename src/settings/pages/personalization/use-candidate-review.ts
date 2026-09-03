import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { useState } from "react";
import type { AgentService } from "../../../services/agent-service";
import type { MemoryScopeKind, MemoryType } from "../../../types/personalization";
import type { ReviewCandidateInput } from "../../../types/personalization-memory";

/** The code the native command layer sends; the message is the whole contract across the wire. */
const CONFLICT_CODE = "personalization-revision-conflict";

export const candidatesQueryKey = ["personalization", "candidates"] as const;

export interface CandidateEdits {
  name: string;
  description: string;
  content: string;
  memoryType: Exclude<MemoryType, "untyped">;
  scopeKind: MemoryScopeKind | "";
  workspaceKey: string;
  /** `null` keeps the proposed audience; a list -- including an empty one -- replaces it. */
  audienceAgentIds: string[] | null;
}

export function emptyEdits(): CandidateEdits {
  return {
    name: "",
    description: "",
    content: "",
    memoryType: "user",
    scopeKind: "",
    workspaceKey: "",
    audienceAgentIds: null,
  };
}

/**
 * Turns a reviewer's edits into a request, omitting everything they did not touch.
 *
 * Omission matters more than it looks: an absent field keeps what the proposal carried, and
 * sending a value for one the reviewer never opened would record a decision they did not make --
 * most damagingly a scope, which would widen a workspace memory to every project.
 */
export function reviewRequestFrom(
  candidateId: string,
  edits: CandidateEdits,
  proposed: { name: string | null; description: string | null; content: string | null },
): ReviewCandidateInput {
  const changed = <T,>(value: T, original: T | null) =>
    value !== (original ?? "") ? value : undefined;
  return {
    candidateId,
    action: "approve-with-edits",
    name: changed(edits.name, proposed.name),
    description: changed(edits.description, proposed.description),
    content: changed(edits.content, proposed.content),
    memoryType: edits.memoryType,
    scopeKind: edits.scopeKind === "" ? undefined : edits.scopeKind,
    workspaceKey: edits.scopeKind === "workspace" ? edits.workspaceKey || undefined : undefined,
    audienceAgentIds: edits.audienceAgentIds ?? undefined,
  };
}

export function useCandidateReview(service: AgentService) {
  const queryClient = useQueryClient();
  const [conflictId, setConflictId] = useState<string | null>(null);
  const [failedId, setFailedId] = useState<string | null>(null);

  const candidatesQuery = useQuery({
    queryKey: candidatesQueryKey,
    queryFn: () => service.listPersonalizationCandidates(),
  });

  const reviewMutation = useMutation({
    mutationFn: (input: ReviewCandidateInput) => service.reviewPersonalizationCandidate(input),
    onMutate: (input) => {
      setConflictId(null);
      setFailedId(null);
      return input;
    },
    onSuccess: () => {
      void queryClient.invalidateQueries({ queryKey: candidatesQueryKey });
      void queryClient.invalidateQueries({ queryKey: ["personalization", "memories"] });
      void queryClient.invalidateQueries({ queryKey: ["personalization", "overview"] });
    },
    onError: (error: Error, input) => {
      // A merge races the record it merges into. Refreshing the queue would hide the candidate
      // before the reviewer saw why nothing happened, so the queue is left exactly as it was.
      if (error.message.includes(CONFLICT_CODE)) setConflictId(input.candidateId);
      else setFailedId(input.candidateId);
    },
  });

  return {
    candidates: candidatesQuery.data ?? [],
    isLoading: candidatesQuery.isPending,
    loadError: candidatesQuery.error,
    isReviewing: reviewMutation.isPending,
    conflictId,
    failedId,
    review: (input: ReviewCandidateInput) => reviewMutation.mutate(input),
    refresh: () => void queryClient.invalidateQueries({ queryKey: candidatesQueryKey }),
  };
}
