import { describe, expect, it } from "vitest";
import { normalizeFolderOpeners, normalizeFolderOpenerPreferences } from "./folder-opener";

describe("folder opener contracts", () => {
  it("normalizes a strict catalog entry", () => {
    expect(normalizeFolderOpeners([{ id: "vscode", category: "editor", status: "available", executablePath: "D:/Code.exe", iconKey: "vscode" }]))
      .toEqual([{ id: "vscode", category: "editor", status: "available", executablePath: "D:/Code.exe", version: null, edition: null, detectionSource: null, iconKey: "vscode", reason: null }]);
  });

  it("rejects unknown ids and invalid preference invariants", () => {
    expect(() => normalizeFolderOpeners([{ id: "unknown", category: "editor", status: "available" }])).toThrow();
    expect(() => normalizeFolderOpenerPreferences({ configuredDefaultOpenerId: "vscode", effectiveDefaultOpenerId: "vscode", enabledOpenerIds: ["vscode"], fallbackActive: false })).toThrow();
  });

  it("deduplicates enabled ids while retaining Explorer", () => {
    expect(normalizeFolderOpenerPreferences({ configuredDefaultOpenerId: "vscode", effectiveDefaultOpenerId: "vscode", enabledOpenerIds: ["vscode", "file-explorer", "vscode"], fallbackActive: false }).enabledOpenerIds)
      .toEqual(["vscode", "file-explorer"]);
  });
});
