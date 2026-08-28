// @vitest-environment jsdom

import { screen, waitFor, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { describe, expect, it, vi } from "vitest";
import "../../../i18n";
import { createAgentServiceDouble, renderWithAppProviders } from "../../../test/render";
import type { EffectivePreview } from "../../../types/personalization";
import type { MemoryDetail, MemoryPage, MemorySummary } from "../../../types/personalization-memory";
import { MemoryListSection } from "./memory-list-section";
import { RuntimePreviewSection } from "./runtime-preview-section";

const SECRET = "sk-live-4d2f9a17c0b84e3f";

function summary(id: string, name: string): MemorySummary {
  return {
    id,
    name,
    description: `description for ${id}`,
    memoryType: "user",
    scopeKind: "global",
    workspaceKey: null,
    status: "active",
    source: "explicit_user",
    sensitivity: "normal",
    revision: 1,
    updatedAt: "2026-02-01T09:00:00Z",
  };
}

function detail(id: string, content: string): MemoryDetail {
  return {
    id,
    name: "duplicate-name",
    description: `description for ${id}`,
    memoryType: "user",
    content,
    scopeKind: "global",
    workspaceKey: null,
    audienceAgentIds: null,
    status: "active",
    source: "explicit_user",
    sensitivity: "normal",
    revision: 1,
    sourceAgentId: null,
    sourceSessionId: null,
    createdAt: "2026-01-01T09:00:00Z",
    updatedAt: "2026-02-01T09:00:00Z",
  };
}

function renderList() {
  const service = createAgentServiceDouble({
    listPersonalizationAgentCapabilities: async () => [],
    listKnownProjects: async () => [],
    listKnownRemoteWorkspaces: async () => [],
    queryPersonalizationMemories: async (): Promise<MemoryPage> => ({
      // Two records that a user cannot tell apart by name. The store allows it, so the page has to
      // stay usable rather than treating the name as an identity.
      items: [
        summary("mem-0000000000000001", "duplicate-name"),
        summary("mem-0000000000000002", "duplicate-name"),
      ],
      nextCursor: null,
      totalMatched: 2,
    }),
    getPersonalizationMemory: async (id: string) =>
      detail(id, id === "mem-0000000000000001" ? "The first body." : "The second body."),
  });
  return renderWithAppProviders(<MemoryListSection service={service} />);
}

describe("memory surfaces", () => {
  it("keeps two memories with the same name independently openable", async () => {
    renderList();
    await screen.findByTestId("personalization-memory-list");

    await userEvent.click(screen.getByTestId("personalization-memory-open-mem-0000000000000001"));
    await waitFor(() => {
      expect(screen.getByTestId("personalization-detail-body").textContent).toBe("The first body.");
    });

    await userEvent.click(screen.getByTestId("personalization-memory-open-mem-0000000000000002"));
    // A page keyed on the display name would have shown the first body again, or refused to open
    // the second row at all.
    await waitFor(() => {
      expect(screen.getByTestId("personalization-detail-body").textContent).toBe("The second body.");
    });
  });

  it("marks which of two identically named rows is open", async () => {
    renderList();
    await screen.findByTestId("personalization-memory-list");

    await userEvent.click(screen.getByTestId("personalization-memory-open-mem-0000000000000002"));

    await waitFor(() => {
      expect(
        screen.getByTestId("personalization-memory-open-mem-0000000000000002").getAttribute("aria-expanded"),
      ).toBe("true");
    });
    expect(
      screen.getByTestId("personalization-memory-open-mem-0000000000000001").getAttribute("aria-expanded"),
    ).toBe("false");
  });

  it("opens a row from the keyboard alone", async () => {
    renderList();
    await screen.findByTestId("personalization-memory-list");

    const row = screen.getByTestId("personalization-memory-open-mem-0000000000000001");
    row.focus();
    await userEvent.keyboard("{Enter}");

    // Every row is a button, so it is reachable by Tab and activates on Enter without a pointer.
    await waitFor(() => {
      expect(screen.getByTestId("personalization-detail")).toBeTruthy();
    });
  });

  it("puts focus in the reset dialog and closes it on Escape", async () => {
    const service = createAgentServiceDouble({
      listPersonalizationAgentCapabilities: async () => [],
      listKnownProjects: async () => [],
      listKnownRemoteWorkspaces: async () => [],
      queryPersonalizationMemories: async (): Promise<MemoryPage> => ({
        items: [],
        nextCursor: null,
        totalMatched: 0,
      }),
      previewPersonalizationReset: async () => ({
        confirmationToken: "token",
        matched: 1,
        global: 1,
        workspace: 0,
        candidates: 0,
        malformed: 0,
      }),
    });
    renderWithAppProviders(<MemoryListSection service={service} />);

    await userEvent.click(await screen.findByTestId("personalization-reset-open"));
    const phrase = await screen.findByTestId("personalization-reset-phrase");

    // The phrase box is the one control the dialog exists for, so it takes focus on open.
    await waitFor(() => {
      expect(document.activeElement).toBe(phrase);
    });

    await userEvent.keyboard("{Escape}");
    await waitFor(() => {
      expect(screen.queryByTestId("personalization-reset-form")).toBeNull();
    });
  });

  it("renders only what the preview redacted, never the value behind it", async () => {
    const previewEffectivePersonalization = vi.fn(
      async (): Promise<EffectivePreview> => ({
        // Carries the secret here so the assertion below has something to catch: the token is
        // opaque state for a later write, and a screen that rendered it would be leaking
        // whatever the native side happened to encode into it.
        revisionToken: `1:agent:standard:${SECRET}`,
        instructionMode: "append",
        includedInstructions: [
          {
            field: "about_user",
            scopeKind: "global",
            scopeKey: "",
            policyRevision: 4,
            mergeAction: "appended",
            // What the native side redacted. The real text held a credential the user had pasted
            // into their own instructions, and the preview is not the place it comes back.
            redactedText: "My key is [REDACTED]",
            characters: 40,
          },
        ],
        excludedInstructions: [],
        memoryDelivery: "none",
        memoryRead: false,
        explicitSave: false,
        automaticExtraction: false,
        candidateCreation: false,
        retrievalWrite: false,
        eligibleMemoryCount: 0,
        consideredMemoryCount: 0,
        memoryExclusions: [],
        warnings: [],
        approximateTokens: 10,
        knownCharacters: 40,
        selectedBodyBudgetMax: 5,
        excludedSurfaces: [],
        estimatorVersion: "test",
        cliInternalCompactionManaged: false,
      }),
    );
    const service = createAgentServiceDouble({
      listPersonalizationAgentCapabilities: async () => [
        {
          agentId: "synthetic-lab-agent",
          displayName: "Synthetic Lab Agent",
          supportsCustomInstructions: true,
          supportsMemoryIndex: false,
          supportsSelectedMemoryBodies: false,
          supportsAutomaticExtraction: false,
        },
      ],
      listKnownProjects: async () => [],
      listKnownRemoteWorkspaces: async () => [],
      previewEffectivePersonalization,
    });
    renderWithAppProviders(<RuntimePreviewSection service={service} />);

    const select = await screen.findByTestId("personalization-preview-agent");
    await within(select).findByText("Synthetic Lab Agent");
    await userEvent.selectOptions(select, "synthetic-lab-agent");

    const included = await screen.findByTestId("personalization-preview-included");
    expect(included.textContent).toContain("[REDACTED]");
    // A settings screen gets screenshotted into issues. The redacted text is the only instruction
    // string rendered, and no other field of the resolution reaches the DOM.
    expect(document.body.textContent).not.toContain(SECRET);
  });
});
