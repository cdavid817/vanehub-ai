import { describe, expect, it } from "vitest";
import {
  canSave,
  dirtyIds,
  discardDraft,
  enterCustomMode,
  isDirty,
  markSaved,
  mergeProfiles,
  restoreInherited,
  setCustomInput,
  setSelection,
  type CliParameterDraftMap,
} from "./draft-state";
import {
  cliParameterCatalogVersion,
  defaultCliParameterSelections,
  editableCliParameterDefinitions,
} from "../../services/cli-parameter-registry";
import { renderCliParameterSegments } from "../../services/cli-parameter-renderer";
import type { ManagedCliAgentId } from "../../types/agent";
import type { CliParameterSelections } from "../../types/cli-parameter";
import type { CliParameterProfile } from "../../types/cli-parameter-profile";

function profile(
  agentId: ManagedCliAgentId,
  overrides: Partial<CliParameterProfile> = {},
): CliParameterProfile {
  const definitions = editableCliParameterDefinitions(agentId);
  const selections: CliParameterSelections =
    overrides.selections ?? defaultCliParameterSelections(agentId);
  return {
    agentId,
    catalogVersion: cliParameterCatalogVersion,
    revision: 0,
    updatedAt: null,
    installation: { installed: true, runnable: true, conflict: false, version: "1.0.0" },
    fields: definitions.map((definition) => ({
      definition,
      support: { state: "supported" },
      optionSupport: {},
    })),
    selections,
    savedPreviews: {
      chat: renderCliParameterSegments(definitions, selections, "chat"),
      interactive: renderCliParameterSegments(definitions, selections, "interactive"),
    },
    diagnostics: [],
    ...overrides,
  };
}

function seeded(): CliParameterDraftMap {
  return mergeProfiles({}, [profile("claude-code"), profile("codex-cli")]);
}

describe("CLI parameter draft state", () => {
  it("starts clean and reports nothing dirty", () => {
    const map = seeded();

    expect(isDirty(map["claude-code"]!)).toBe(false);
    expect(canSave(map["claude-code"]!)).toBe(false);
    expect(map["claude-code"]!.baselineCatalogVersion).toBe(cliParameterCatalogVersion);
  });

  it("keys transient custom text by agent as well as parameter", () => {
    // Both CLIs have a `model` parameter. v1's page shared one text box between them, so typing a
    // Claude model produced a Codex draft nobody asked for.
    let map = seeded();
    map = enterCustomMode(map, "claude-code", "model", "");
    map = setCustomInput(map, "claude-code", "model", "claude-opus-5");

    expect(map["claude-code"]!.customInputs.model).toBe("claude-opus-5");
    expect(map["codex-cli"]!.customInputs.model).toBeUndefined();
    expect(map["codex-cli"]!.selections.model).toEqual({ state: "inherit" });
  });

  it("treats choosing Custom as an editor change, not a value change", () => {
    let map = seeded();
    map = enterCustomMode(map, "claude-code", "model", "");

    expect(map["claude-code"]!.selections.model).toEqual({ state: "inherit" });
    expect(isDirty(map["claude-code"]!)).toBe(false);
    // Empty custom input is a local validation state, so save stays refused.
    expect(map["claude-code"]!.invalidIds).toContain("model");
    expect(canSave(map["claude-code"]!)).toBe(false);
  });

  it("does not write an empty value when the custom box is cleared", () => {
    let map = seeded();
    map = enterCustomMode(map, "claude-code", "model", "");
    map = setCustomInput(map, "claude-code", "model", "claude-opus-5");
    expect(canSave(map["claude-code"]!)).toBe(true);

    map = setCustomInput(map, "claude-code", "model", "   ");

    // The last valid value is still what would be transported; the field is merely invalid.
    expect(map["claude-code"]!.selections.model).toEqual({
      state: "value",
      value: "claude-opus-5",
    });
    expect(map["claude-code"]!.invalidIds).toContain("model");
    expect(canSave(map["claude-code"]!)).toBe(false);
  });

  it("never turns a cleared custom box into inherit on its own", () => {
    let map = seeded();
    map = setSelection(map, "claude-code", "model", { state: "value", value: "opus" });
    map = enterCustomMode(map, "claude-code", "model", "opus");
    map = setCustomInput(map, "claude-code", "model", "");

    expect(map["claude-code"]!.selections.model).toEqual({ state: "value", value: "opus" });
  });

  it("replaces an untouched draft on refetch and keeps a dirty one", () => {
    let map = seeded();
    const moved = profile("codex-cli", {
      revision: 3,
      selections: {
        ...defaultCliParameterSelections("codex-cli"),
        model: { state: "value", value: "gpt-5.5" },
      },
    });

    map = mergeProfiles(map, [moved]);
    expect(map["codex-cli"]!.selections.model).toEqual({ state: "value", value: "gpt-5.5" });
    expect(map["codex-cli"]!.baselineRevision).toBe(3);

    map = setSelection(map, "codex-cli", "search", { state: "value", value: true });
    map = mergeProfiles(map, [moved]);
    expect(map["codex-cli"]!.selections.search).toEqual({ state: "value", value: true });
    expect(map["codex-cli"]!.conflict).toBe("none");
  });

  it("marks a revision conflict rather than overwriting a dirty draft", () => {
    let map = seeded();
    map = setSelection(map, "claude-code", "safeMode", { state: "value", value: true });

    map = mergeProfiles(map, [profile("claude-code", { revision: 7 })]);

    expect(map["claude-code"]!.conflict).toBe("revision");
    expect(map["claude-code"]!.selections.safeMode).toEqual({ state: "value", value: true });
    expect(canSave(map["claude-code"]!)).toBe(false);
  });

  it("marks a catalog conflict distinctly from a revision conflict", () => {
    let map = seeded();
    map = setSelection(map, "claude-code", "safeMode", { state: "value", value: true });

    map = mergeProfiles(map, [profile("claude-code", { catalogVersion: "3.0.0", revision: 7 })]);

    expect(map["claude-code"]!.conflict).toBe("catalog");
  });

  it("discards a draft back to its baseline without touching the server", () => {
    let map = seeded();
    map = setSelection(map, "claude-code", "safeMode", { state: "value", value: true });
    map = enterCustomMode(map, "claude-code", "model", "");

    map = discardDraft(map, "claude-code");

    expect(isDirty(map["claude-code"]!)).toBe(false);
    expect(map["claude-code"]!.customMode).toEqual([]);
    expect(map["claude-code"]!.invalidIds).toEqual([]);
  });

  it("restores every editable parameter to inherit", () => {
    let map = seeded();
    map = setSelection(map, "claude-code", "safeMode", { state: "value", value: true });

    map = restoreInherited(map, "claude-code", editableCliParameterDefinitions("claude-code"));

    expect(
      Object.values(map["claude-code"]!.selections).every((entry) => entry.state === "inherit"),
    ).toBe(true);
  });

  it("adopts the saved profile as the new baseline", () => {
    let map = seeded();
    map = setSelection(map, "claude-code", "safeMode", { state: "value", value: true });
    expect(dirtyIds(map["claude-code"]!)).toEqual(["safeMode"]);

    map = markSaved(
      map,
      profile("claude-code", {
        revision: 1,
        selections: {
          ...defaultCliParameterSelections("claude-code"),
          safeMode: { state: "value", value: true },
        },
      }),
    );

    expect(isDirty(map["claude-code"]!)).toBe(false);
    expect(map["claude-code"]!.baselineRevision).toBe(1);
  });

  it("keeps drafts for every CLI while only one is on screen", () => {
    let map = seeded();
    map = setSelection(map, "claude-code", "safeMode", { state: "value", value: true });
    map = setSelection(map, "codex-cli", "search", { state: "value", value: true });

    expect(dirtyIds(map["claude-code"]!)).toEqual(["safeMode"]);
    expect(dirtyIds(map["codex-cli"]!)).toEqual(["search"]);
  });
});
