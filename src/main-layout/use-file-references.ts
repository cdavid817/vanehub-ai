import { useCallback, useState } from "react";
import { fileReferenceId, type MentionLineRange } from "../services/composer-mention";
import { MAX_CHAT_FILE_REFERENCES, type ChatFileReference } from "../types/chat";
import type { FileSearchMatch } from "../types/session-workspace";

/**
 * Draft file references for the composer. Identity is (path, range): the same file at two
 * ranges is two distinct references, so neither attaching nor removing can be keyed on the
 * path alone.
 */
export function useFileReferences(onLimitReached?: () => void) {
  const [fileReferences, setFileReferences] = useState<ChatFileReference[]>([]);
  const addFileReference = useCallback((candidate: FileSearchMatch, range: MentionLineRange) => {
    setFileReferences((current) => {
      const id = fileReferenceId(candidate.path, range);
      if (current.some((reference) => reference.id === id)) return current;
      // Refusing here rather than letting the domain reject the whole message at send
      // time: the limit is easy to reach by dragging, and finding out only on send means
      // losing the attempt with a message that was never localized.
      if (current.length >= MAX_CHAT_FILE_REFERENCES) {
        onLimitReached?.();
        return current;
      }
      return [...current, { id, path: candidate.path, name: candidate.name, ...range }];
    });
  }, [onLimitReached]);
  const removeFileReference = useCallback((referenceId: string) => {
    setFileReferences((current) => current.filter((reference) => reference.id !== referenceId));
  }, []);
  return { addFileReference, fileReferences, removeFileReference, setFileReferences };
}
