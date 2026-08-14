import { useCallback, useState } from "react";
import { fileReferenceId, type MentionLineRange } from "../services/composer-mention";
import type { ChatFileReference } from "../types/chat";
import type { FileSearchMatch } from "../types/session-workspace";

/**
 * Draft file references for the composer. Identity is (path, range): the same file at two
 * ranges is two distinct references, so neither attaching nor removing can be keyed on the
 * path alone.
 */
export function useFileReferences() {
  const [fileReferences, setFileReferences] = useState<ChatFileReference[]>([]);
  const addFileReference = useCallback((candidate: FileSearchMatch, range: MentionLineRange) => {
    setFileReferences((current) => {
      const id = fileReferenceId(candidate.path, range);
      return current.some((reference) => reference.id === id)
        ? current
        : [...current, { id, path: candidate.path, name: candidate.name, ...range }];
    });
  }, []);
  const removeFileReference = useCallback((referenceId: string) => {
    setFileReferences((current) => current.filter((reference) => reference.id !== referenceId));
  }, []);
  return { addFileReference, fileReferences, removeFileReference, setFileReferences };
}
