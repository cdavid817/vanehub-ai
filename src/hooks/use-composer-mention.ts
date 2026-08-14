import { useMemo } from "react";
import type { SeatMentionOption } from "../components/chat/SeatMentionCompletion";
import {
  composerMentionToken,
  parseComposerMention,
  replaceComposerMention,
  type MentionLineRange,
} from "../services/composer-mention";
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
  const token = composerMentionToken(value);
  const parsed = token === null ? null : parseComposerMention(token);
  const mentionQuery = parsed?.path.toLowerCase() ?? null;
  const mentionRange: MentionLineRange = { startLine: parsed?.startLine, endLine: parsed?.endLine };
  const fileSuggestions = useMemo(() => {
    if (mentionQuery === null || disabled) return [];
    // Only an identical attachment is hidden: the same file at a different range is a
    // distinct reference and must stay offerable.
    const selected = new Set(fileReferences.map((reference) => reference.id));
    return fileReferenceCandidates
      .filter((candidate) => !selected.has(candidate.path))
      .slice(0, SUGGESTION_LIMIT);
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
    // A range already typed before a candidate is picked must survive the selection,
    // otherwise choosing from completion would silently discard it.
    mentionRange,
    applyMention: (insertion: string) => replaceComposerMention(value, insertion),
  };
}
