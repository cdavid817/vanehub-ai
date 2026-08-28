// @vitest-environment jsdom

import { screen, waitFor } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";
import "../../../i18n";
import { createAgentServiceDouble, renderWithAppProviders } from "../../../test/render";
import type { MemoryCandidate, MemoryPage, MemorySummary } from "../../../types/personalization-memory";
import { CandidateReviewSection } from "./candidate-review-section";
import { MemoryListSection } from "./memory-list-section";

/**
 * The threshold above which measured virtualization would be worth its cost.
 *
 * Below it, normal document flow is the cheaper and more correct choice: a virtualized list breaks
 * find-in-page, anchor links and screen-reader row counts, and it is only worth those losses when
 * the alternative is rendering thousands of nodes.
 */
const VIRTUALIZATION_THRESHOLD = 500;

function summary(index: number): MemorySummary {
  return {
    id: `mem-${String(index).padStart(16, "0")}`,
    name: `memory-${index}`,
    description: "",
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

function candidate(index: number): MemoryCandidate {
  return {
    id: `cnd-${String(index).padStart(16, "0")}`,
    kind: "create",
    name: `candidate-${index}`,
    description: "",
    memoryType: "user",
    content: "",
    targetId: null,
    expectedTargetRevision: null,
    source: "onepiece_automatic",
    sourceAgentId: "onepiece",
    sourceSessionId: null,
    sourceMessageId: null,
    createdAt: "2026-02-01T09:00:00Z",
  };
}

describe("rendered row bounds", () => {
  it("asks for a page far below the threshold however many memories exist", async () => {
    const queryPersonalizationMemories = vi.fn(async (query): Promise<MemoryPage> => {
      // A store with far more than the threshold. The page size is what bounds the render, and it
      // is the request that has to carry it -- trimming after the fact would still have read them.
      const limit = query.limit ?? 5_000;
      return {
        items: Array.from({ length: limit }, (_, index) => summary(index)),
        nextCursor: "cursor-2",
        totalMatched: 5_000,
      };
    });
    const service = createAgentServiceDouble({
      listPersonalizationAgentCapabilities: async () => [],
      listKnownProjects: async () => [],
      listKnownRemoteWorkspaces: async () => [],
      queryPersonalizationMemories,
    });
    renderWithAppProviders(<MemoryListSection service={service} />);

    const list = await screen.findByTestId("personalization-memory-list");
    await waitFor(() => {
      expect(list.querySelectorAll("li").length).toBeGreaterThan(0);
    });
    expect(list.querySelectorAll("li").length).toBeLessThan(VIRTUALIZATION_THRESHOLD);
    expect(queryPersonalizationMemories.mock.calls[0][0].limit).toBeLessThan(VIRTUALIZATION_THRESHOLD);
  });

  it("keeps the review queue below the threshold too", async () => {
    const service = createAgentServiceDouble({
      // The native command clamps this to 200; the queue asks for far less, and the two together
      // are what keep a single render from reaching the threshold.
      listPersonalizationCandidates: async (limit?: number) =>
        Array.from({ length: limit ?? 50 }, (_, index) => candidate(index)),
      listPersonalizationAgentCapabilities: async () => [],
      listKnownProjects: async () => [],
      listKnownRemoteWorkspaces: async () => [],
    });
    renderWithAppProviders(<CandidateReviewSection service={service} />);

    const list = await screen.findByTestId("personalization-review-list");
    expect(list.querySelectorAll("li").length).toBeLessThan(VIRTUALIZATION_THRESHOLD);
  });

  it("renders rows in ordinary document flow", async () => {
    const service = createAgentServiceDouble({
      listPersonalizationAgentCapabilities: async () => [],
      listKnownProjects: async () => [],
      listKnownRemoteWorkspaces: async () => [],
      queryPersonalizationMemories: async (): Promise<MemoryPage> => ({
        items: [summary(1), summary(2)],
        nextCursor: null,
        totalMatched: 2,
      }),
    });
    renderWithAppProviders(<MemoryListSection service={service} />);

    const list = await screen.findByTestId("personalization-memory-list");
    // No spacer elements, no absolute positioning, no windowing container: find-in-page and a
    // screen reader's row count both work because every row that exists is in the document.
    expect(list.querySelectorAll("li").length).toBe(2);
    expect(list.getAttribute("style")).toBeNull();
    expect(list.querySelector("[data-virtualized]")).toBeNull();
  });
});
