/**
 * The drag and clipboard type only this application writes. Paste attaches a reference
 * only when it is present, which is what keeps pasting ordinary text — including text
 * that happens to look like a path — from being reinterpreted.
 */
export const FILE_REFERENCE_TRANSFER_TYPE = "application/x-vanehub-file-path";

/**
 * Clipboard custom formats must carry the `web ` prefix; drag transfers must not. Same
 * meaning, two spellings the platform imposes.
 */
const CLIPBOARD_TRANSFER_TYPE = `web ${FILE_REFERENCE_TRANSFER_TYPE}`;

/** Drag source: the path travels as both the private type and plain text. */
export function writeFileReferenceDrag(transfer: DataTransfer, path: string): void {
  transfer.setData(FILE_REFERENCE_TRANSFER_TYPE, path);
  transfer.setData("text/plain", path);
  transfer.effectAllowed = "copy";
}

export function readFileReferenceTransfer(transfer: DataTransfer | null): string | null {
  if (!transfer) return null;
  const path = transfer.getData(FILE_REFERENCE_TRANSFER_TYPE) || transfer.getData(CLIPBOARD_TRANSFER_TYPE);
  return path.trim() === "" ? null : path;
}

export function transferCarriesFileReference(transfer: DataTransfer | null): boolean {
  return Array.from(transfer?.types ?? []).some(
    (type) => type === FILE_REFERENCE_TRANSFER_TYPE || type === CLIPBOARD_TRANSFER_TYPE,
  );
}

/**
 * Copies a path to the clipboard as both the private type and plain text. Falls back to
 * plain text alone where custom formats are unavailable: the path stays copyable
 * everywhere, and pasting it into the composer simply inserts text, which is what it did
 * before this feature existed.
 */
export async function copyFileReferencePath(path: string): Promise<void> {
  const clipboard = navigator.clipboard;
  if (!clipboard) throw new Error("clipboard unavailable");
  const canWriteCustomFormats = typeof clipboard.write === "function" && typeof ClipboardItem === "function";
  if (!canWriteCustomFormats) {
    await clipboard.writeText(path);
    return;
  }
  try {
    await clipboard.write([
      new ClipboardItem({
        "text/plain": new Blob([path], { type: "text/plain" }),
        [CLIPBOARD_TRANSFER_TYPE]: new Blob([path], { type: CLIPBOARD_TRANSFER_TYPE }),
      }),
    ]);
  } catch {
    await clipboard.writeText(path);
  }
}
