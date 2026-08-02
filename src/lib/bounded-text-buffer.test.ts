import { describe, expect, it } from "vitest";
import { BoundedTextBuffer } from "./bounded-text-buffer";

describe("BoundedTextBuffer", () => {
  it("retains separate large chunks without rebuilding the snapshot", () => {
    const buffer = new BoundedTextBuffer(64 * 1024);
    buffer.append("a".repeat(20 * 1024));
    buffer.append("b".repeat(20 * 1024));

    expect(buffer.chunkCount).toBe(2);
    expect(buffer.byteLength).toBe(40 * 1024);
    expect(buffer.snapshot()).toBe(`${"a".repeat(20 * 1024)}${"b".repeat(20 * 1024)}`);
  });

  it("evicts only old content and keeps valid UTF-8 at the byte limit", () => {
    const buffer = new BoundedTextBuffer(12);
    buffer.append("older");
    buffer.append("你好世界");

    expect(buffer.byteLength).toBeLessThanOrEqual(12);
    expect(buffer.snapshot()).toBe("你好世界");
  });
});
