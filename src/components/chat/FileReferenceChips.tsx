import { FileText, X } from "lucide-react";
import { useTranslation } from "react-i18next";
import type { ChatFileReference } from "../../types/chat";
import { FileReferenceLines } from "./FileReferenceLines";

/**
 * Extracted from `ChatInputBox` (10.15-10.19) purely to make room under the 300-line cap for
 * the new Run Configuration popover wiring — the chip row itself is unchanged, still a narrow,
 * self-contained "list of attached references" with no dependency on composer config state.
 */
export function FileReferenceChips({
  disabled,
  fileReferences,
  isStreaming,
  onRemoveFileReference,
}: {
  disabled?: boolean;
  fileReferences: ChatFileReference[];
  isStreaming: boolean;
  onRemoveFileReference: (referenceId: string) => void;
}) {
  const { t } = useTranslation();
  if (!fileReferences.length) return null;
  return (
    <div className="flex flex-wrap gap-1.5 px-3 pt-3" data-testid="composer-file-references">
      {fileReferences.map((reference) => (
        <span className="inline-flex max-w-full items-center gap-1 rounded-md border border-border bg-muted/60 px-2 py-1 text-xs" key={reference.id}>
          <FileText className="h-3.5 w-3.5 shrink-0 text-primary" aria-hidden="true" />
          <span className="truncate">{reference.name}</span>
          <FileReferenceLines reference={reference} />
          <button className="rounded text-muted-foreground hover:text-foreground" disabled={disabled || isStreaming} onClick={() => onRemoveFileReference(reference.id)} title={t("chat.removeFileReference")} type="button">
            <X className="h-3 w-3" aria-hidden="true" />
          </button>
        </span>
      ))}
    </div>
  );
}
