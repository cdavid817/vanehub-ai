// @vitest-environment jsdom

import { afterEach, describe, expect, it, vi } from "vitest";
import {
  FILE_REFERENCE_TRANSFER_TYPE,
  copyFileReferencePath,
  readFileReferenceTransfer,
  transferCarriesFileReference,
  writeFileReferenceDrag,
} from "./file-reference-transfer";

const CLIPBOARD_TYPE = `web ${FILE_REFERENCE_TRANSFER_TYPE}`;

function fakeTransfer(entries: Record<string, string> = {}): DataTransfer {
  return {
    dropEffect: "none",
    effectAllowed: "none",
    get types() {
      return Object.keys(entries);
    },
    getData: (type: string) => entries[type] ?? "",
    setData: (type: string, value: string) => {
      entries[type] = value;
    },
  } as unknown as DataTransfer;
}

afterEach(() => {
  vi.unstubAllGlobals();
});

describe("file reference transfer", () => {
  it("writes a drag as both the private type and plain text", () => {
    const transfer = fakeTransfer();
    writeFileReferenceDrag(transfer, "src/utils.rs");
    expect(transfer.getData(FILE_REFERENCE_TRANSFER_TYPE)).toBe("src/utils.rs");
    // Plain text keeps the path useful when dropped anywhere else.
    expect(transfer.getData("text/plain")).toBe("src/utils.rs");
    expect(transfer.effectAllowed).toBe("copy");
  });

  it("reads back a drag and a clipboard write", () => {
    expect(readFileReferenceTransfer(fakeTransfer({ [FILE_REFERENCE_TRANSFER_TYPE]: "a.rs" }))).toBe("a.rs");
    expect(readFileReferenceTransfer(fakeTransfer({ [CLIPBOARD_TYPE]: "b.rs" }))).toBe("b.rs");
  });

  it("ignores anything that is not this application's transfer", () => {
    expect(readFileReferenceTransfer(fakeTransfer({ "text/plain": "src/utils.rs" }))).toBeNull();
    expect(readFileReferenceTransfer(fakeTransfer())).toBeNull();
    expect(readFileReferenceTransfer(null)).toBeNull();
    expect(readFileReferenceTransfer(fakeTransfer({ [FILE_REFERENCE_TRANSFER_TYPE]: "   " }))).toBeNull();
  });

  it("detects the transfer type without reading it", () => {
    expect(transferCarriesFileReference(fakeTransfer({ [FILE_REFERENCE_TRANSFER_TYPE]: "a.rs" }))).toBe(true);
    expect(transferCarriesFileReference(fakeTransfer({ [CLIPBOARD_TYPE]: "a.rs" }))).toBe(true);
    // Text that looks exactly like a path is still just text.
    expect(transferCarriesFileReference(fakeTransfer({ "text/plain": "src/utils.rs" }))).toBe(false);
    expect(transferCarriesFileReference(null)).toBe(false);
  });

  it("copies both representations when custom formats are available", async () => {
    const write = vi.fn().mockResolvedValue(undefined);
    vi.stubGlobal("ClipboardItem", class {
      constructor(public readonly items: Record<string, Blob>) {}
    });
    vi.stubGlobal("navigator", { clipboard: { write, writeText: vi.fn() } });

    await copyFileReferencePath("src/utils.rs");

    const item = write.mock.calls[0][0][0] as { items: Record<string, Blob> };
    expect(Object.keys(item.items).sort()).toEqual(["text/plain", CLIPBOARD_TYPE].sort());
  });

  it("falls back to plain text where custom formats are unavailable", async () => {
    const writeText = vi.fn().mockResolvedValue(undefined);
    vi.stubGlobal("ClipboardItem", undefined);
    vi.stubGlobal("navigator", { clipboard: { writeText } });

    await copyFileReferencePath("src/utils.rs");

    // Degrading to plain text keeps the path copyable; pasting it then inserts text,
    // which is what it did before this feature existed.
    expect(writeText).toHaveBeenCalledWith("src/utils.rs");
  });

  it("falls back to plain text when a custom-format write is rejected", async () => {
    const writeText = vi.fn().mockResolvedValue(undefined);
    vi.stubGlobal("ClipboardItem", class {});
    vi.stubGlobal("navigator", { clipboard: { write: vi.fn().mockRejectedValue(new Error("denied")), writeText } });

    await copyFileReferencePath("src/utils.rs");

    expect(writeText).toHaveBeenCalledWith("src/utils.rs");
  });
});
