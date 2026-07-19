import { describe, expect, it } from "vitest";
import { defaultSessionTitleFromPath, folderNameFromPath, normalizeDisplayPath } from "./session-path";

describe("session path helpers", () => {
  it("normalizes Windows extended-length path prefixes for display", () => {
    expect(normalizeDisplayPath("\\\\?\\D:\\cdavid\\Documents\\code\\claude-code")).toBe("D:\\cdavid\\Documents\\code\\claude-code");
  });

  it("derives session names from the current folder and timestamp", () => {
    const date = new Date("2026-07-19T12:34:56");
    expect(folderNameFromPath("\\\\?\\D:\\cdavid\\Documents\\code\\claude-code")).toBe("claude-code");
    expect(defaultSessionTitleFromPath("\\\\?\\D:\\cdavid\\Documents\\code\\claude-code", date)).toBe("claude-code-20260719-123456");
  });
});
