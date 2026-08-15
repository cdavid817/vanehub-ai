import { useQuery } from "@tanstack/react-query";
import { useDebouncedValue } from "../hooks/use-debounced-value";
import { composerMentionQuery } from "../services/composer-mention";
import { agentService } from "../services/runtime-agent-client";
import type { FileSearchMatch } from "../types/session-workspace";

const MENTION_SEARCH_DEBOUNCE_MS = 150;
const MENTION_SEARCH_LIMIT = 8;
const UNAVAILABLE = {
  context: { availability: "unavailable" as const, rootName: null, reason: null },
  items: [],
  truncated: false,
};

/**
 * Mention candidates come from a ranked native search, not from the Documents tab listing:
 * that listing is bound to Markdown and text by its own requirement and cannot serve code
 * files. Debouncing keeps a burst of keystrokes to one native walk; distinct query keys keep
 * a slow response for a shorter prefix from overwriting a later, more specific one.
 */
export function useMentionCandidates(sessionId: string | null, draft: string): FileSearchMatch[] {
  const mentionQuery = composerMentionQuery(draft);
  const debouncedQuery = useDebouncedValue(mentionQuery ?? "", MENTION_SEARCH_DEBOUNCE_MS);
  const search = useQuery({
    enabled: Boolean(sessionId) && mentionQuery !== null,
    queryKey: ["session-file-search", sessionId, debouncedQuery],
    queryFn: () => sessionId
      ? agentService.searchSessionFiles(sessionId, debouncedQuery, MENTION_SEARCH_LIMIT)
      : Promise.resolve(UNAVAILABLE),
  });
  // Closing the completion must drop stale results rather than leave the panel populated.
  return mentionQuery === null ? [] : search.data?.items ?? [];
}
