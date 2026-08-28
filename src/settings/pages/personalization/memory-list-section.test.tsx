// @vitest-environment jsdom

import { screen, waitFor, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { describe, expect, it, vi } from "vitest";
import "../../../i18n";
import { createAgentServiceDouble, renderWithAppProviders } from "../../../test/render";
import type { MemoryPage, MemoryQuery, MemorySummary } from "../../../types/personalization-memory";
import { MemoryListSection } from "./memory-list-section";

function summary(overrides: Partial<MemorySummary> = {}): MemorySummary {
  return {
    id: "mem-0000000000000001",
    name: "prefers-metric-units",
    description: "Uses metric units in explanations.",
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

function renderList(pages: (query: MemoryQuery) => MemoryPage) {
  const queryPersonalizationMemories = vi.fn(async (query: MemoryQuery) => pages(query));
  const service = createAgentServiceDouble({
    listPersonalizationAgentCapabilities: async () => [
      {
        agentId: "synthetic-lab-agent",
        displayName: "Synthetic Lab Agent",
        supportsCustomInstructions: true,
        supportsMemoryIndex: true,
        supportsSelectedMemoryBodies: false,
        supportsAutomaticExtraction: false,
      },
    ],
    listKnownProjects: async () => [
      { path: "/code/vanehub", displayName: "vanehub", isGit: true, lastOpenedAt: "2026-01-01T00:00:00Z" },
    ],
    listKnownRemoteWorkspaces: async () => [],
    resolvePersonalizationWorkspace: async () => ({ workspaceKey: "ws-1", kind: "local" as const }),
    queryPersonalizationMemories,
  });
  const rendered = renderWithAppProviders(<MemoryListSection service={service} />);
  return { ...rendered, queryPersonalizationMemories };
}

const ONE_PAGE = (): MemoryPage => ({ items: [summary()], nextCursor: null, totalMatched: 1 });

describe("MemoryListSection", () => {
  it("asks for a bounded page rather than everything", async () => {
    const { queryPersonalizationMemories } = renderList(ONE_PAGE);

    await waitFor(() => {
      expect(queryPersonalizationMemories).toHaveBeenCalledWith(
        expect.objectContaining({ limit: 25, cursor: undefined }),
      );
    });
  });

  it("renders names and metadata without any body", async () => {
    renderList(() => ({
      items: [summary({ description: "Uses metric units in explanations." })],
      nextCursor: null,
      totalMatched: 1,
    }));

    const row = await screen.findByTestId("personalization-memory-row-mem-0000000000000001");

    expect(within(row).getByText("prefers-metric-units")).toBeTruthy();
    expect(within(row).getByText("Uses metric units in explanations.")).toBeTruthy();
    // A summary has no body to render, and the row must not invent a place to put one.
    expect(row.textContent).not.toContain("24-hour time");
  });

  it("sends each filter the user sets", async () => {
    const { queryPersonalizationMemories } = renderList(ONE_PAGE);
    await screen.findByTestId("personalization-memory-filters");

    await userEvent.type(screen.getByTestId("personalization-memory-search"), "npm");
    await userEvent.selectOptions(screen.getByTestId("personalization-memory-status"), "archived");
    await userEvent.selectOptions(screen.getByTestId("personalization-memory-type"), "project");
    const sourceAgent = screen.getByTestId("personalization-memory-source-agent");
    await within(sourceAgent).findByText("Synthetic Lab Agent");
    await userEvent.selectOptions(sourceAgent, "synthetic-lab-agent");

    await waitFor(() => {
      expect(queryPersonalizationMemories).toHaveBeenCalledWith(
        expect.objectContaining({
          text: "npm",
          status: "archived",
          memoryType: "project",
          sourceAgentId: "synthetic-lab-agent",
        }),
      );
    });
  });

  it("keeps who recorded a memory apart from who may read it", async () => {
    const { queryPersonalizationMemories } = renderList(ONE_PAGE);
    const select = await screen.findByTestId("personalization-memory-audience-agent");
    // The options come from the registry, so they arrive after the first render.
    await within(select).findByText("Synthetic Lab Agent");

    await userEvent.selectOptions(select, "synthetic-lab-agent");

    // One Agent recording something another reads is ordinary; a single control would make that
    // memory unfindable from one of the two sides.
    await waitFor(() => {
      const last = queryPersonalizationMemories.mock.calls.at(-1)?.[0];
      expect(last).toMatchObject({ audienceAgentId: "synthetic-lab-agent" });
      expect(last?.sourceAgentId).toBeUndefined();
    });
  });

  it("selects a workspace with the workspace scope rather than filtering on nothing", async () => {
    const { queryPersonalizationMemories } = renderList(ONE_PAGE);
    await screen.findByTestId("personalization-memory-filters");

    await userEvent.selectOptions(screen.getByTestId("personalization-memory-scope"), "workspace");

    await waitFor(() => {
      expect(queryPersonalizationMemories).toHaveBeenCalledWith(
        expect.objectContaining({ scopeKind: "workspace", workspaceKey: "ws-1" }),
      );
    });
    expect(screen.getByTestId("personalization-memory-workspace")).toBeTruthy();
  });

  it("pages forward with the cursor the page issued", async () => {
    const { queryPersonalizationMemories } = renderList((query) =>
      query.cursor
        ? { items: [summary({ id: "mem-0000000000000002", name: "second" })], nextCursor: null, totalMatched: 2 }
        : { items: [summary()], nextCursor: "cursor-2", totalMatched: 2 },
    );

    await screen.findByTestId("personalization-memory-row-mem-0000000000000001");
    await userEvent.click(screen.getByTestId("personalization-memory-next"));

    await waitFor(() => {
      expect(screen.getByTestId("personalization-memory-row-mem-0000000000000002")).toBeTruthy();
    });
    expect(queryPersonalizationMemories).toHaveBeenCalledWith(
      expect.objectContaining({ cursor: "cursor-2" }),
    );
  });

  it("starts the result set over when a filter changes mid-scroll", async () => {
    const { queryPersonalizationMemories } = renderList((query) =>
      query.cursor
        ? { items: [summary({ id: "mem-0000000000000002" })], nextCursor: null, totalMatched: 2 }
        : { items: [summary()], nextCursor: "cursor-2", totalMatched: 2 },
    );

    await screen.findByTestId("personalization-memory-row-mem-0000000000000001");
    await userEvent.click(screen.getByTestId("personalization-memory-next"));
    await screen.findByTestId("personalization-memory-row-mem-0000000000000002");

    await userEvent.selectOptions(screen.getByTestId("personalization-memory-status"), "archived");

    // Carrying the cursor into a different filtered set resumes from a row no longer in it, which
    // reads as a page of missing results.
    await waitFor(() => {
      const last = queryPersonalizationMemories.mock.calls.at(-1)?.[0];
      expect(last?.cursor).toBeUndefined();
      expect(last?.status).toBe("archived");
    });
  });

  it("reports what this page holds when the store cannot count the rest cheaply", async () => {
    renderList(() => ({ items: [summary(), summary({ id: "mem-0000000000000002" })], nextCursor: null, totalMatched: null }));

    await waitFor(() => {
      expect(screen.getByTestId("personalization-memory-count").textContent).toContain("本页 2 条");
    });
  });

  it("says the list is unreadable rather than showing it as empty", async () => {
    renderList(() => {
      throw new Error("personalization-storage-unavailable");
    });

    await waitFor(() => {
      expect(screen.getByTestId("personalization-memory-error")).toBeTruthy();
    });
    expect(screen.queryByTestId("personalization-memory-empty")).toBeNull();
  });

  it("says nothing matched when the filters exclude everything", async () => {
    renderList(() => ({ items: [], nextCursor: null, totalMatched: 0 }));

    await waitFor(() => {
      expect(screen.getByTestId("personalization-memory-empty")).toBeTruthy();
    });
  });
});
