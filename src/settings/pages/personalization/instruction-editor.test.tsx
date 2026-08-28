// @vitest-environment jsdom

import { screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { describe, expect, it, vi } from "vitest";
import "../../../i18n";
import { createAgentServiceDouble, renderWithAppProviders } from "../../../test/render";
import type { PersonalizationPolicy, PersonalizationPolicyPatch } from "../../../types/personalization";
import { customInstructionsFieldCharacterLimit } from "../../../types/settings";
import { PersonalizationInstructionsView } from "./instructions-view";

const STORED: PersonalizationPolicy = {
  scopeKind: "global",
  scopeKey: "",
  revision: 4,
  instructionMergeMode: "append",
  aboutUser: "Backend engineer.",
  styleRules: "Lead with the conclusion.",
  memoryReadMode: "enabled",
  explicitSaveMode: "enabled",
  automaticExtractionMode: "enabled",
  globalMemoryAccessMode: "enabled",
};

function renderEditor(overrides: Parameters<typeof createAgentServiceDouble>[0] = {}) {
  const patchPersonalizationPolicy = vi.fn(async (patch: PersonalizationPolicyPatch) => ({
    ...STORED,
    revision: STORED.revision + 1,
    aboutUser: patch.aboutUser ?? STORED.aboutUser,
    styleRules: patch.styleRules ?? STORED.styleRules,
  }));
  const service = createAgentServiceDouble({
    listPersonalizationAgentCapabilities: async () => [],
    listKnownProjects: async () => [],
    listKnownRemoteWorkspaces: async () => [],
    getPersonalizationPolicy: async () => STORED,
    patchPersonalizationPolicy,
    ...overrides,
  });
  const rendered = renderWithAppProviders(<PersonalizationInstructionsView service={service} />);
  return { ...rendered, patchPersonalizationPolicy };
}

async function typeInto(field: string, text: string) {
  const input = await screen.findByTestId(`personalization-field-${field}`);
  await userEvent.clear(input);
  await userEvent.type(input, text);
  return input;
}

describe("instruction editor", () => {
  it("writes nothing while the user types", async () => {
    const { patchPersonalizationPolicy } = renderEditor();

    await typeInto("aboutUser", "Half a sen");
    await userEvent.tab();

    // A blur-save writes on every focus change, so a half-finished sentence and a stray Tab both
    // reach the store, and there is no moment at which the user said the text was ready.
    expect(patchPersonalizationPolicy).not.toHaveBeenCalled();
    expect(screen.getByTestId("personalization-dirty")).toBeTruthy();
  });

  it("saves with the revision the user was looking at", async () => {
    const { patchPersonalizationPolicy } = renderEditor();

    await typeInto("styleRules", "Answer in Chinese.");
    await userEvent.click(screen.getByTestId("personalization-save"));

    await waitFor(() => {
      expect(patchPersonalizationPolicy).toHaveBeenCalledWith(
        expect.objectContaining({ scopeKind: "global", expectedRevision: 4, styleRules: "Answer in Chinese." }),
      );
    });
  });

  it("creates a never-written layer without claiming a revision", async () => {
    const { patchPersonalizationPolicy } = renderEditor({ getPersonalizationPolicy: async () => null });

    await typeInto("aboutUser", "First write.");
    await userEvent.click(screen.getByTestId("personalization-save"));

    // Sending 0 would claim the caller saw a revision that does not exist, and the store would
    // refuse a first save forever.
    await waitFor(() => {
      expect(patchPersonalizationPolicy).toHaveBeenCalledWith(
        expect.objectContaining({ expectedRevision: undefined }),
      );
    });
  });

  it("refuses to save a field past the character limit", async () => {
    const { patchPersonalizationPolicy } = renderEditor();

    const input = await screen.findByTestId("personalization-field-aboutUser");
    await userEvent.clear(input);
    // `paste` rather than `type`: 3001 keystrokes is minutes of test time for the same state.
    await userEvent.click(input);
    await userEvent.paste("a".repeat(customInstructionsFieldCharacterLimit + 1));

    await waitFor(() => {
      expect(screen.getByTestId("personalization-count-aboutUser").textContent).toContain("3001");
    });
    expect(screen.getByTestId("personalization-save").hasAttribute("disabled")).toBe(true);
    expect(patchPersonalizationPolicy).not.toHaveBeenCalled();
  });

  it("puts the text back the way the store has it", async () => {
    renderEditor();

    const input = await typeInto("aboutUser", "Something else.");
    await userEvent.click(screen.getByTestId("personalization-discard"));

    await waitFor(() => {
      expect((input as HTMLTextAreaElement).value).toBe("Backend engineer.");
    });
    expect(screen.queryByTestId("personalization-dirty")).toBeNull();
  });

  it("keeps the typed text when the save fails", async () => {
    renderEditor({
      patchPersonalizationPolicy: async () => {
        throw new Error("personalization-storage-unavailable");
      },
    });

    const input = await typeInto("styleRules", "Worth keeping.");
    await userEvent.click(screen.getByTestId("personalization-save"));

    await waitFor(() => {
      expect(screen.getByTestId("personalization-save-error")).toBeTruthy();
    });
    // The text the user typed is the only copy of it there is.
    expect((input as HTMLTextAreaElement).value).toBe("Worth keeping.");
  });

  it("counts tokens from both fields at the same rate the native estimator uses", async () => {
    renderEditor();

    await typeInto("aboutUser", "12345678");
    await waitFor(() => {
      // 8 + 25 characters over 4 characters per token, rounded up.
      expect(screen.getByTestId("personalization-tokens").textContent).toContain("9");
    });
  });

  it("offers nothing to save until something changes", async () => {
    renderEditor();

    await screen.findByTestId("personalization-instruction-editor");

    expect(screen.getByTestId("personalization-save").hasAttribute("disabled")).toBe(true);
    expect(screen.getByTestId("personalization-discard").hasAttribute("disabled")).toBe(true);
  });

  it("says a layer it could not read is not safe to edit", async () => {
    renderEditor({
      getPersonalizationPolicy: async () => {
        throw new Error("personalization-storage-unavailable");
      },
    });

    await waitFor(() => {
      expect(screen.getByTestId("personalization-editor-error")).toBeTruthy();
    });
    expect(screen.queryByTestId("personalization-instruction-editor")).toBeNull();
  });
});
