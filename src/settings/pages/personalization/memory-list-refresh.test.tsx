// @vitest-environment jsdom

import { screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { describe, expect, it, vi } from "vitest";
import "../../../i18n";
import { createAgentServiceDouble, renderWithAppProviders } from "../../../test/render";
import type { MemoryPage, MemoryQuery, MemorySummary } from "../../../types/personalization-memory";
import { MemoryListSection } from "./memory-list-section";

function summary(overrides: Partial<MemorySummary> = {}): MemorySummary {
  return {
    id: "mem-0000000000000001",
    name: "first-memory",
    description: "The row that was already on screen.",
    memoryType: "user",
    scopeKind: "global",
    workspaceKey: null,
    status: "active",
    source: "explicit_user",
    sensitivity: "normal",
    revision: 1,
    updatedAt: "2026-02-01T09:00:00Z",
    ...overrides,
  };
}

/** A store whose next answer can be held open, which is the only way to observe a refresh. */
function holdableStore() {
  let release: ((page: MemoryPage) => void) | null = null;
  const getPersonalizationMemory = vi.fn(async () => null);
  const queryPersonalizationMemories = vi.fn(async (query: MemoryQuery): Promise<MemoryPage> => {
    if (!query.status) {
      return { items: [summary()], nextCursor: null, totalMatched: 1 };
    }
    return new Promise<MemoryPage>((resolve) => {
      release = resolve;
    });
  });

  return {
    getPersonalizationMemory,
    queryPersonalizationMemories,
    releaseWith: (page: MemoryPage) => release?.(page),
    isHeld: () => release !== null,
    service: createAgentServiceDouble({
      listPersonalizationAgentCapabilities: async () => [],
      listKnownProjects: async () => [],
      listKnownRemoteWorkspaces: async () => [],
      queryPersonalizationMemories,
      getPersonalizationMemory,
    }),
  };
}

describe("memory list refresh behaviour", () => {
  it("keeps the previous rows on screen while the next result loads", async () => {
    const world = holdableStore();
    renderWithAppProviders(<MemoryListSection service={world.service} />);
    await screen.findByTestId("personalization-memory-row-mem-0000000000000001");

    await userEvent.selectOptions(screen.getByTestId("personalization-memory-status"), "archived");
    await waitFor(() => {
      expect(world.isHeld()).toBe(true);
    });

    // Blanking here makes the page flicker through an empty state the user's data never had.
    expect(screen.getByTestId("personalization-memory-row-mem-0000000000000001")).toBeTruthy();
    expect(screen.queryByTestId("personalization-memory-empty")).toBeNull();
  });

  it("says the rows are the previous result while they are", async () => {
    const world = holdableStore();
    renderWithAppProviders(<MemoryListSection service={world.service} />);
    await screen.findByTestId("personalization-memory-row-mem-0000000000000001");

    expect(screen.getByTestId("personalization-memory-refresh-status").textContent).toBe("");

    await userEvent.selectOptions(screen.getByTestId("personalization-memory-status"), "archived");

    // Stale rows shown without a word are read as the result of the filter just set.
    await waitFor(() => {
      expect(screen.getByTestId("personalization-memory-refresh-status").textContent).toContain(
        "上一次的结果",
      );
    });

    world.releaseWith({ items: [summary({ id: "mem-0000000000000002", name: "second-memory" })], nextCursor: null, totalMatched: 1 });

    await waitFor(() => {
      expect(screen.getByTestId("personalization-memory-refresh-status").textContent).toBe("");
    });
    expect(screen.getByTestId("personalization-memory-row-mem-0000000000000002")).toBeTruthy();
  });

  it("announces the refresh politely rather than interrupting", async () => {
    const world = holdableStore();
    renderWithAppProviders(<MemoryListSection service={world.service} />);
    await screen.findByTestId("personalization-memory-row-mem-0000000000000001");

    // It changes on every keystroke in the search box; an assertive region would make the list
    // unusable with a screen reader.
    expect(
      screen.getByTestId("personalization-memory-refresh-status").getAttribute("aria-live"),
    ).toBe("polite");
  });

  it("never reads a body to render the list", async () => {
    const world = holdableStore();
    renderWithAppProviders(<MemoryListSection service={world.service} />);
    await screen.findByTestId("personalization-memory-row-mem-0000000000000001");

    await userEvent.type(screen.getByTestId("personalization-memory-search"), "first");
    await userEvent.click(screen.getByTestId("personalization-memory-next"));

    // A per-row detail call would restore exactly the cost this list was built to remove, one
    // request at a time instead of one big one.
    expect(world.getPersonalizationMemory).not.toHaveBeenCalled();
  });
});
