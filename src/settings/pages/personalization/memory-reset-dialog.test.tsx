// @vitest-environment jsdom

import { screen, waitFor, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { describe, expect, it, vi } from "vitest";
import "../../../i18n";
import { createAgentServiceDouble, renderWithAppProviders } from "../../../test/render";
import type { MaintenanceResult, MemoryPage, ResetPreview, ResetScope } from "../../../types/personalization-memory";
import { MemoryListSection } from "./memory-list-section";

function preview(overrides: Partial<ResetPreview> = {}): ResetPreview {
  return {
    confirmationToken: "token-any",
    matched: 3,
    global: 2,
    workspace: 1,
    candidates: 2,
    malformed: 1,
    ...overrides,
  };
}

function outcome(overrides: Partial<MaintenanceResult> = {}): MaintenanceResult {
  return {
    matched: 3,
    deletedFiles: 3,
    removedProjectionRows: 3,
    revokedRetrievalEntries: 3,
    quarantined: 0,
    failures: [],
    ...overrides,
  };
}

function renderDialogHost(overrides: Parameters<typeof createAgentServiceDouble>[0] = {}) {
  const previewPersonalizationReset = vi.fn(async (scope: ResetScope) =>
    preview({ confirmationToken: `token-${scope.scopeKind ?? "any"}-${scope.includeArchived}` }),
  );
  const executePersonalizationReset = vi.fn(async () => outcome());
  const service = createAgentServiceDouble({
    listPersonalizationAgentCapabilities: async () => [],
    listKnownProjects: async () => [
      { path: "/code/vanehub", displayName: "vanehub", isGit: true, lastOpenedAt: "2026-01-01T00:00:00Z" },
    ],
    listKnownRemoteWorkspaces: async () => [],
    resolvePersonalizationWorkspace: async () => ({ workspaceKey: "ws-1", kind: "local" as const }),
    queryPersonalizationMemories: async (): Promise<MemoryPage> => ({
      items: [],
      nextCursor: null,
      totalMatched: 0,
    }),
    previewPersonalizationReset,
    executePersonalizationReset,
    ...overrides,
  });
  const rendered = renderWithAppProviders(<MemoryListSection service={service} />);
  return { ...rendered, previewPersonalizationReset, executePersonalizationReset };
}

async function openDialog() {
  await userEvent.click(await screen.findByTestId("personalization-reset-open"));
  return screen.findByTestId("personalization-reset-form");
}

describe("MemoryResetDialog", () => {
  it("shows exactly what would be removed, including what a count could hide", async () => {
    renderDialogHost();
    await openDialog();

    const counts = await screen.findByTestId("personalization-reset-counts");
    // Pending proposals and unreadable files go too; omitting them would understate the loss.
    expect(counts.textContent).toContain("3");
    expect(counts.textContent).toContain("2");
    expect(counts.textContent).toContain("1");
  });

  it("refuses to delete until the phrase is typed exactly", async () => {
    renderDialogHost();
    await openDialog();
    await screen.findByTestId("personalization-reset-counts");

    expect(screen.getByTestId("personalization-reset-execute").hasAttribute("disabled")).toBe(true);

    await userEvent.type(screen.getByTestId("personalization-reset-phrase"), "delete");
    expect(screen.getByTestId("personalization-reset-execute").hasAttribute("disabled")).toBe(true);

    await userEvent.clear(screen.getByTestId("personalization-reset-phrase"));
    await userEvent.type(screen.getByTestId("personalization-reset-phrase"), "DELETE");
    await waitFor(() => {
      expect(screen.getByTestId("personalization-reset-execute").hasAttribute("disabled")).toBe(false);
    });
  });

  it("deletes with the token the preview on screen issued", async () => {
    const world = renderDialogHost();
    await openDialog();
    await screen.findByTestId("personalization-reset-counts");

    await userEvent.type(screen.getByTestId("personalization-reset-phrase"), "DELETE");
    await userEvent.click(screen.getByTestId("personalization-reset-execute"));

    await waitFor(() => {
      expect(world.executePersonalizationReset).toHaveBeenCalledWith(
        { includeArchived: false },
        "token-any-false",
        "DELETE",
      );
    });
  });

  it("re-previews and clears the typed phrase when the selection changes", async () => {
    const world = renderDialogHost();
    await openDialog();
    await screen.findByTestId("personalization-reset-counts");

    await userEvent.type(screen.getByTestId("personalization-reset-phrase"), "DELETE");
    await userEvent.click(screen.getByTestId("personalization-reset-archived"));

    // Keeping the phrase would let a user confirm one scope and delete another without retyping.
    await waitFor(() => {
      expect((screen.getByTestId("personalization-reset-phrase") as HTMLInputElement).value).toBe("");
    });
    await waitFor(() => {
      expect(world.previewPersonalizationReset).toHaveBeenCalledWith({ includeArchived: true });
    });
  });

  it("never carries a token from one scope into another", async () => {
    const world = renderDialogHost();
    await openDialog();
    await screen.findByTestId("personalization-reset-counts");

    await userEvent.selectOptions(screen.getByTestId("personalization-reset-scope"), "global");
    await waitFor(() => {
      expect(world.previewPersonalizationReset).toHaveBeenCalledWith(
        expect.objectContaining({ scopeKind: "global" }),
      );
    });
    await userEvent.type(screen.getByTestId("personalization-reset-phrase"), "DELETE");
    await userEvent.click(screen.getByTestId("personalization-reset-execute"));

    await waitFor(() => {
      expect(world.executePersonalizationReset).toHaveBeenCalledWith(
        expect.objectContaining({ scopeKind: "global" }),
        "token-global-false",
        "DELETE",
      );
    });
  });

  it("asks for a workspace before previewing a workspace reset", async () => {
    const world = renderDialogHost({ listKnownProjects: async () => [] });
    await openDialog();
    await screen.findByTestId("personalization-reset-counts");
    world.previewPersonalizationReset.mockClear();

    await userEvent.selectOptions(screen.getByTestId("personalization-reset-scope"), "workspace");

    await waitFor(() => {
      expect(screen.getByTestId("personalization-reset-needs-workspace")).toBeTruthy();
    });
    expect(world.previewPersonalizationReset).not.toHaveBeenCalled();
  });

  it("reports what happened on every surface", async () => {
    renderDialogHost();
    await openDialog();
    await screen.findByTestId("personalization-reset-counts");

    await userEvent.type(screen.getByTestId("personalization-reset-phrase"), "DELETE");
    await userEvent.click(screen.getByTestId("personalization-reset-execute"));

    const result = await screen.findByTestId("personalization-reset-result");
    expect(within(result).queryByTestId("personalization-reset-partial")).toBeNull();
    expect(screen.getByTestId("personalization-reset-complete")).toBeTruthy();
  });

  it("says which surfaces were not cleared rather than reporting success", async () => {
    renderDialogHost({
      executePersonalizationReset: async () =>
        outcome({ removedProjectionRows: 1, failures: ["sqlite-projection"] }),
    });
    await openDialog();
    await screen.findByTestId("personalization-reset-counts");

    await userEvent.type(screen.getByTestId("personalization-reset-phrase"), "DELETE");
    await userEvent.click(screen.getByTestId("personalization-reset-execute"));

    // A user told the reset succeeded while a projection row survived would believe a memory is
    // gone that a runtime can still recall.
    const partial = await screen.findByTestId("personalization-reset-partial");
    expect(partial.textContent).toContain("索引行");
    expect(screen.queryByTestId("personalization-reset-complete")).toBeNull();
  });

  it("offers nothing to delete when nothing matched", async () => {
    renderDialogHost({ previewPersonalizationReset: async () => preview({ matched: 0 }) });
    await openDialog();
    await screen.findByTestId("personalization-reset-counts");

    await userEvent.type(screen.getByTestId("personalization-reset-phrase"), "DELETE");

    expect(screen.getByTestId("personalization-reset-execute").hasAttribute("disabled")).toBe(true);
  });

  it("says nothing was deleted when the store refuses", async () => {
    renderDialogHost({
      executePersonalizationReset: async () => {
        throw new Error("personalization-reset-refused: token-expired");
      },
    });
    await openDialog();
    await screen.findByTestId("personalization-reset-counts");

    await userEvent.type(screen.getByTestId("personalization-reset-phrase"), "DELETE");
    await userEvent.click(screen.getByTestId("personalization-reset-execute"));

    await waitFor(() => {
      expect(screen.getByTestId("personalization-reset-failed")).toBeTruthy();
    });
    expect(screen.queryByTestId("personalization-reset-result")).toBeNull();
  });
});
