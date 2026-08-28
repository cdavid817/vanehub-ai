// @vitest-environment jsdom

import { fireEvent, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { describe, expect, it, vi } from "vitest";
import "../../../i18n";
import { createAgentServiceDouble, renderWithAppProviders } from "../../../test/render";
import { SettingsProvider } from "../../settings-provider";
import type { PersonalizationPolicy } from "../../../types/personalization";
import { PersonalizationPage } from "../personalization-page";
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

async function renderPage() {
  const rendered = renderWithAppProviders(
    <SettingsProvider>
      <PersonalizationPage />
    </SettingsProvider>,
  );
  await screen.findByRole("tablist");
  return rendered;
}

function renderEditor(overrides: Parameters<typeof createAgentServiceDouble>[0] = {}) {
  const patchPersonalizationPolicy = vi.fn(async () => ({ ...STORED, revision: 5 }));
  const service = createAgentServiceDouble({
    listPersonalizationAgentCapabilities: async () => [],
    listKnownProjects: async () => [],
    listKnownRemoteWorkspaces: async () => [],
    listPersonalizationPolicies: async () => [STORED],
    getPersonalizationPolicy: async () => STORED,
    patchPersonalizationPolicy,
    ...overrides,
  });
  const rendered = renderWithAppProviders(<PersonalizationInstructionsView service={service} />);
  return { ...rendered, patchPersonalizationPolicy };
}

describe("personalization accessibility", () => {
  it("keeps the tablist to one tab stop", async () => {
    await renderPage();

    // Four stops would make Tab walk every destination before reaching the panel, which is the
    // thing the user came to use.
    expect(screen.getByTestId("personalization-view-tab-overview").getAttribute("tabindex")).toBe("0");
    for (const view of ["instructions", "memory", "runtimePreview"]) {
      expect(screen.getByTestId(`personalization-view-tab-${view}`).getAttribute("tabindex")).toBe("-1");
    }
  });

  it("moves between destinations with the arrow keys", async () => {
    await renderPage();
    screen.getByTestId("personalization-view-tab-overview").focus();

    await userEvent.keyboard("{ArrowRight}");

    await waitFor(() => {
      expect(screen.getByTestId("personalization-view-tab-instructions").getAttribute("aria-selected")).toBe("true");
    });
    // Selection follows focus, so focus has to follow selection: otherwise the panel changes while
    // the keyboard stays behind on the old tab.
    expect(document.activeElement).toBe(screen.getByTestId("personalization-view-tab-instructions"));
  });

  it("wraps around and jumps to the ends", async () => {
    await renderPage();
    screen.getByTestId("personalization-view-tab-overview").focus();

    await userEvent.keyboard("{ArrowLeft}");
    await waitFor(() => {
      expect(screen.getByTestId("personalization-view-tab-runtimePreview").getAttribute("aria-selected")).toBe("true");
    });

    await userEvent.keyboard("{Home}");
    await waitFor(() => {
      expect(screen.getByTestId("personalization-view-tab-overview").getAttribute("aria-selected")).toBe("true");
    });

    await userEvent.keyboard("{End}");
    await waitFor(() => {
      expect(screen.getByTestId("personalization-view-tab-runtimePreview").getAttribute("aria-selected")).toBe("true");
    });
  });

  it("ties each destination to the panel it controls", async () => {
    await renderPage();

    const tab = screen.getByTestId("personalization-view-tab-overview");
    const panel = screen.getByRole("tabpanel");

    expect(tab.getAttribute("aria-controls")).toBe(panel.getAttribute("id"));
    expect(panel.getAttribute("aria-labelledby")).toBe(tab.getAttribute("id"));
    expect(panel.getAttribute("tabindex")).toBe("0");
  });

  it("names every instruction control for a screen reader", async () => {
    renderEditor();

    await screen.findByTestId("personalization-instruction-editor");

    expect(screen.getByLabelText("关于你")).toBeTruthy();
    expect(screen.getByLabelText("回复风格")).toBeTruthy();
    expect(screen.getByLabelText("本层的合并方式")).toBeTruthy();
    expect(screen.getByLabelText("层")).toBeTruthy();
  });

  it("points each field at the counter that describes it", async () => {
    renderEditor();

    const field = await screen.findByTestId("personalization-field-aboutUser");
    const counter = screen.getByTestId("personalization-count-aboutUser");

    expect(field.getAttribute("aria-describedby")).toBe(counter.getAttribute("id"));
    expect(field.getAttribute("aria-invalid")).toBe("false");
  });

  it("marks a field over the limit as invalid", async () => {
    renderEditor();

    const field = await screen.findByTestId("personalization-field-aboutUser");
    await userEvent.click(field);
    await userEvent.paste("a".repeat(3001));

    await waitFor(() => {
      expect(field.getAttribute("aria-invalid")).toBe("true");
    });
  });

  it("saves on the keyboard without reaching for the button", async () => {
    const { patchPersonalizationPolicy } = renderEditor();

    const field = await screen.findByTestId("personalization-field-aboutUser");
    await userEvent.clear(field);
    await userEvent.type(field, "Typed.");
    fireEvent.keyDown(field, { ctrlKey: true, key: "s" });

    await waitFor(() => {
      expect(patchPersonalizationPolicy).toHaveBeenCalledTimes(1);
    });
  });

  it("does not save while an input method is composing", async () => {
    const { patchPersonalizationPolicy } = renderEditor();

    const field = await screen.findByTestId("personalization-field-aboutUser");
    await userEvent.clear(field);
    await userEvent.type(field, "Half compo");

    // Confirming a candidate reaches the same handler. Saving here would store half a word the
    // user has not finished choosing.
    fireEvent.keyDown(field, { ctrlKey: true, key: "s", isComposing: true });
    fireEvent.keyDown(field, { ctrlKey: true, key: "s", keyCode: 229 });

    // Let anything those two could have started actually run. An immediate negative assertion
    // passes even with the guard removed, because the save does not reach the service in the same
    // tick as the key event.
    await new Promise((resolve) => {
      setTimeout(resolve, 10);
    });
    expect(patchPersonalizationPolicy).not.toHaveBeenCalled();

    // The same chord does save once composition is over, so it was the guard that stopped it and
    // not something incidental about this editor's state.
    fireEvent.keyDown(field, { ctrlKey: true, key: "s" });
    await waitFor(() => {
      expect(patchPersonalizationPolicy).toHaveBeenCalledTimes(1);
    });
  });

  it("keeps composed text exactly as the input method produced it", async () => {
    renderEditor();

    const field = (await screen.findByTestId("personalization-field-aboutUser")) as HTMLTextAreaElement;
    fireEvent.compositionStart(field);
    fireEvent.change(field, { target: { value: "に" } });
    fireEvent.change(field, { target: { value: "日本" } });
    fireEvent.compositionEnd(field, { data: "日本語" });
    fireEvent.change(field, { target: { value: "日本語" } });

    // Nothing here rewrites the value, which is what breaks composition when a field normalizes
    // input mid-compose.
    await waitFor(() => {
      expect(field.value).toBe("日本語");
    });
    expect(screen.getByTestId("personalization-count-aboutUser").textContent).toContain("3");
  });

  it("moves focus to a conflict the user has to answer", async () => {
    let stored = STORED;
    renderEditor({
      getPersonalizationPolicy: async () => stored,
      patchPersonalizationPolicy: async () => {
        stored = { ...STORED, revision: 9, aboutUser: "Theirs." };
        throw new Error("personalization-revision-conflict: expected 4, stored 9");
      },
    });

    const field = await screen.findByTestId("personalization-field-aboutUser");
    await userEvent.clear(field);
    await userEvent.type(field, "Mine.");
    await userEvent.click(screen.getByTestId("personalization-save"));

    const conflict = await screen.findByTestId("personalization-conflict");
    // Otherwise a keyboard user is left on a Save button that has just gone dead, with no way to
    // find out why except tabbing forward and guessing.
    await waitFor(() => {
      expect(document.activeElement).toBe(conflict);
    });
  });

  it("announces the unsaved marker politely rather than interrupting", async () => {
    renderEditor();

    const field = await screen.findByTestId("personalization-field-aboutUser");
    await userEvent.type(field, "!");

    const marker = await screen.findByTestId("personalization-dirty");
    expect(marker.parentElement?.getAttribute("aria-live")).toBe("polite");
  });
});
