import { describe, expect, it } from "vitest";
import { classifyLayoutTier } from "./use-layout-tier";

describe("classifyLayoutTier", () => {
  it.each([
    [1600, "wide"],
    [1280, "wide"],
    [1279, "standard"],
    [1024, "standard"],
    [1023, "compact"],
    [768, "compact"],
    [767, "narrow"],
    [640, "narrow"],
    [0, "narrow"],
  ] as const)("classifies %ipx as %s", (width, expected) => {
    expect(classifyLayoutTier(width)).toBe(expected);
  });
});
