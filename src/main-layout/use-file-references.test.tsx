// @vitest-environment jsdom

import { act, renderHook } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";
import { MAX_CHAT_FILE_REFERENCES } from "../types/chat";
import { useFileReferences } from "./use-file-references";

const utils = { name: "utils.rs", path: "src/utils.rs" };

describe("useFileReferences", () => {
  it("deduplicates a dropped or pasted path against an identical typed reference", () => {
    const { result } = renderHook(() => useFileReferences());
    // Drop and paste attach with an empty range, exactly as a mention typed without one
    // does, so both land on the same identity and the second is a no-op.
    act(() => result.current.addFileReference(utils, {}));
    act(() => result.current.addFileReference(utils, {}));
    expect(result.current.fileReferences).toHaveLength(1);
    expect(result.current.fileReferences[0].id).toBe("src/utils.rs");
  });

  it("keeps a whole-file reference distinct from a ranged one to the same path", () => {
    const { result } = renderHook(() => useFileReferences());
    act(() => result.current.addFileReference(utils, {}));
    act(() => result.current.addFileReference(utils, { startLine: 10, endLine: 20 }));
    expect(result.current.fileReferences.map((reference) => reference.id)).toEqual([
      "src/utils.rs",
      "src/utils.rs:10-20",
    ]);
  });

  it("refuses to attach past the reference ceiling and says so once", () => {
    const onLimitReached = vi.fn();
    const { result } = renderHook(() => useFileReferences(onLimitReached));

    for (let index = 0; index < MAX_CHAT_FILE_REFERENCES; index += 1) {
      act(() => result.current.addFileReference({ name: `f${index}.rs`, path: `src/f${index}.rs` }, {}));
    }
    expect(result.current.fileReferences).toHaveLength(MAX_CHAT_FILE_REFERENCES);
    expect(onLimitReached).not.toHaveBeenCalled();

    act(() => result.current.addFileReference({ name: "extra.rs", path: "src/extra.rs" }, {}));
    // Refused here rather than at send time, where the domain's rejection would surface
    // as an untranslated message and cost the user the whole attempt.
    expect(result.current.fileReferences).toHaveLength(MAX_CHAT_FILE_REFERENCES);
    expect(onLimitReached).toHaveBeenCalledTimes(1);
  });

  it("does not treat an already-attached duplicate as hitting the ceiling", () => {
    const onLimitReached = vi.fn();
    const { result } = renderHook(() => useFileReferences(onLimitReached));
    act(() => result.current.addFileReference(utils, {}));
    act(() => result.current.addFileReference(utils, {}));
    expect(onLimitReached).not.toHaveBeenCalled();
  });

  it("removes by identity, leaving the other region attached", () => {
    const { result } = renderHook(() => useFileReferences());
    act(() => result.current.addFileReference(utils, { startLine: 10, endLine: 20 }));
    act(() => result.current.addFileReference(utils, { startLine: 50, endLine: 60 }));
    act(() => result.current.removeFileReference("src/utils.rs:10-20"));
    expect(result.current.fileReferences.map((reference) => reference.id)).toEqual([
      "src/utils.rs:50-60",
    ]);
  });
});
