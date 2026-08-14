import { useMemo } from "react";
import type { SeatMentionOption } from "../components/chat/SeatMentionCompletion";
import { composerMentionQuery, replaceComposerMention } from "../services/composer-mention";
import type { ChatFileReference } from "../types/chat";
import type { FileSearchMatch } from "../types/session-workspace";

const SUGGESTION_LIMIT = 8;

/**
 * Composer completion state. File candidates arrive already ranked and bounded by the
 * native runtime, so this only removes what is already attached; participant candidates
 * are in-memory and still filtered here.
 */
export function useComposerMention({
  disabled,
  fileReferenceCandidates,
  fileReferences,
  participantMentions,
  value,
}: {
  disabled?: boolean;
  fileReferenceCandidates: FileSearchMatch[];
  fileReferences: ChatFileReference[];
  participantMentions: SeatMentionOption[];
  value: string;
}) {
  const mentionQuery = composerMentionQuery(value);
  const fileSuggestions = useMemo(() => {
    if (mentionQuery === null || disabled) return [];
    const selected = new Set(fileReferences.map((reference) => reference.path));
    return fileReferenceCandidates.filter((candidate) => !selected.has(candidate.path)).slice(0, SUGGESTION_LIMIT);
  }, [disabled, fileReferenceCandidates, fileReferences, mentionQuery]);
  const participantSuggestions = useMemo(() => {
    if (mentionQuery === null || disabled) return [];
    return participantMentions
      .filter((option) => `${option.mention} ${option.roleName ?? ""} ${option.agentName}`.toLowerCase().includes(mentionQuery))
      .slice(0, SUGGESTION_LIMIT);
  }, [disabled, mentionQuery, participantMentions]);
  return {
    fileSuggestions,
    participantSuggestions,
    applyMention: (insertion: string) => replaceComposerMention(value, insertion),
  };
}
