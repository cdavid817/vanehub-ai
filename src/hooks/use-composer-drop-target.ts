import { useState, type ClipboardEvent, type DragEvent } from "react";
import {
  readFileReferenceTransfer,
  transferCarriesFileReference,
} from "../services/file-reference-transfer";

/**
 * Accepts a workspace path dropped or pasted from the Files tab. Both gestures carry a
 * file, not a region, so both attach a whole-file reference.
 *
 * Anything not carrying this application's own transfer type falls through untouched —
 * that is what keeps ordinary dragging and ordinary pasting working, including pasting
 * text that happens to look like a path.
 */
export function useComposerDropTarget({
  disabled,
  onAttachPath,
}: {
  disabled: boolean;
  onAttachPath: (path: string) => void;
}) {
  // `dragleave` also fires when the pointer crosses into a child element, so a boolean
  // here would make the affordance flicker across the textarea's own children.
  const [dragDepth, setDragDepth] = useState(0);

  function accepts(transfer: DataTransfer | null): boolean {
    return !disabled && transferCarriesFileReference(transfer);
  }

  function attachFrom(transfer: DataTransfer | null): void {
    const path = readFileReferenceTransfer(transfer);
    if (path !== null) onAttachPath(path);
  }

  return {
    isDropTarget: dragDepth > 0,
    handlers: {
      onDragEnter: (event: DragEvent<HTMLElement>) => {
        if (!accepts(event.dataTransfer)) return;
        event.preventDefault();
        setDragDepth((current) => current + 1);
      },
      onDragLeave: (event: DragEvent<HTMLElement>) => {
        if (!accepts(event.dataTransfer)) return;
        setDragDepth((current) => Math.max(0, current - 1));
      },
      onDragOver: (event: DragEvent<HTMLElement>) => {
        if (!accepts(event.dataTransfer)) return;
        event.preventDefault();
        event.dataTransfer.dropEffect = "copy";
      },
      onDrop: (event: DragEvent<HTMLElement>) => {
        setDragDepth(0);
        if (!accepts(event.dataTransfer)) return;
        event.preventDefault();
        attachFrom(event.dataTransfer);
      },
      onPaste: (event: ClipboardEvent<HTMLElement>) => {
        // Not preventing default unless this is our own transfer: a disabled composer or
        // any other clipboard content must still paste the way it always did.
        if (!accepts(event.clipboardData)) return;
        event.preventDefault();
        attachFrom(event.clipboardData);
      },
    },
  };
}
