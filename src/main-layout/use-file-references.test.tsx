// @vitest-environment jsdom

import { act, renderHook } from "@testing-library/react";
import { describe, expect, it } from "vitest";
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
