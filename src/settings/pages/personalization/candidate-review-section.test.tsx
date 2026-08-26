// @vitest-environment jsdom

import { screen, waitFor, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { describe, expect, it, vi } from "vitest";
import "../../../i18n";
import { createAgentServiceDouble, renderWithAppProviders } from "../../../test/render";
import type { MemoryCandidate, ReviewCandidateInput } from "../../../types/personalization-memory";
import { CandidateReviewSection } from "./candidate-review-section";

const CREATE_ID = "cnd-0000000000000001";
const UPDATE_ID = "cnd-0000000000000002";

function created(overrides: Partial<MemoryCandidate> = {}): MemoryCandidate {
  return {
    id: CREATE_ID,
    kind: "create",
    name: "prefers-vitest-watch",
    description: "Runs vitest in watch mode.",
    memoryType: "feedback",
    content: "The user keeps vitest --watch running.",
    targetId: null,
    expectedTargetRevision: null,
    source: "onepiece_automatic",
    sourceAgentId: "onepiece",
    sourceSessionId: "session-1",
    sourceMessageId: "message-8",
    createdAt: "2026-02-01T09:00:00Z",
    ...overrides,
  };
}

function updates(): MemoryCandidate {
  return created({
    id: UPDATE_ID,
    kind: "update",
    name: null,
    description: null,
    memoryType: null,
    content: "npm only; pnpm breaks the katex chunk split.",
    targetId: "mem-0000000000000002",
    expectedTargetRevision: 2,
    source: "cli_automatic",
  });
}

function renderQueue(overrides: Parameters<typeof createAgentServiceDouble>[0] = {}) {
  const reviewPersonalizationCandidate = vi.fn(async (input: ReviewCandidateInput) => ({
    candidateId: input.candidateId,
    status: "approved" as const,
    resultingMemoryId: "mem-0000000000000009",
    retainedContent: true,
  }));
  const service = createAgentServiceDouble({
    listPersonalizationCandidates: async () => [created(), updates()],
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
    reviewPersonalizationCandidate,
    ...overrides,
  });
  const rendered = renderWithAppProviders(<CandidateReviewSection service={service} />);
  return { ...rendered, reviewPersonalizationCandidate };
}

describe("CandidateReviewSection", () => {
  it("shows what an Agent proposed without storing it", async () => {
    renderQueue();

    const candidate = await screen.findByTestId(`personalization-candidate-${CREATE_ID}`);
    expect(within(candidate).getByText("prefers-vitest-watch")).toBeTruthy();
    expect(
      within(candidate).getByTestId(`personalization-candidate-content-preview-${CREATE_ID}`).textContent,
    ).toContain("vitest --watch");
  });

  it("approves as proposed", async () => {
    const world = renderQueue();
    await screen.findByTestId(`personalization-candidate-${CREATE_ID}`);

    await userEvent.click(screen.getByTestId(`personalization-candidate-approve-${CREATE_ID}`));

    await waitFor(() => {
      expect(world.reviewPersonalizationCandidate).toHaveBeenCalledWith({
        candidateId: CREATE_ID,
        action: "approve",
      });
    });
  });

  it("rejects without touching anything active", async () => {
    const world = renderQueue();
    await screen.findByTestId(`personalization-candidate-${CREATE_ID}`);

    await userEvent.click(screen.getByTestId(`personalization-candidate-reject-${CREATE_ID}`));

    await waitFor(() => {
      expect(world.reviewPersonalizationCandidate).toHaveBeenCalledWith({
        candidateId: CREATE_ID,
        action: "reject",
      });
    });
  });

  it("sends only the fields the reviewer actually changed", async () => {
    const world = renderQueue();
    await screen.findByTestId(`personalization-candidate-${CREATE_ID}`);

    await userEvent.click(screen.getByTestId(`personalization-candidate-edit-${CREATE_ID}`));
    const content = screen.getByTestId(`personalization-candidate-content-${CREATE_ID}`);
    await userEvent.clear(content);
    await userEvent.type(content, "Keeps a watch-mode test run open.");
    await userEvent.click(screen.getByTestId(`personalization-candidate-approve-edits-${CREATE_ID}`));

    await waitFor(() => {
      const sent = world.reviewPersonalizationCandidate.mock.calls.at(-1)?.[0];
      expect(sent?.content).toBe("Keeps a watch-mode test run open.");
      // Untouched fields are omitted, so the proposal's own values stand.
      expect(sent?.name).toBeUndefined();
      expect(sent?.description).toBeUndefined();
    });
  });

  it("keeps the proposed scope unless the reviewer chooses one", async () => {
    const world = renderQueue();
    await screen.findByTestId(`personalization-candidate-${CREATE_ID}`);

    await userEvent.click(screen.getByTestId(`personalization-candidate-edit-${CREATE_ID}`));
    await userEvent.click(screen.getByTestId(`personalization-candidate-approve-edits-${CREATE_ID}`));

    // Preselecting global would quietly widen a workspace memory to every project on an edit that
    // only changed the wording.
    await waitFor(() => {
      const sent = world.reviewPersonalizationCandidate.mock.calls.at(-1)?.[0];
      expect(sent?.scopeKind).toBeUndefined();
      expect(sent?.workspaceKey).toBeUndefined();
      expect(sent?.audienceAgentIds).toBeUndefined();
    });
  });

  it("changes the scope when the reviewer picks one", async () => {
    const world = renderQueue();
    await screen.findByTestId(`personalization-candidate-${CREATE_ID}`);

    await userEvent.click(screen.getByTestId(`personalization-candidate-edit-${CREATE_ID}`));
    await userEvent.selectOptions(
      screen.getByTestId(`personalization-candidate-scope-${CREATE_ID}`),
      "workspace",
    );
    await userEvent.click(screen.getByTestId(`personalization-candidate-approve-edits-${CREATE_ID}`));

    await waitFor(() => {
      expect(world.reviewPersonalizationCandidate).toHaveBeenCalledWith(
        expect.objectContaining({ scopeKind: "workspace", workspaceKey: "ws-1" }),
      );
    });
  });

  it("narrows the audience to the Agents the reviewer ticked", async () => {
    const world = renderQueue();
    await screen.findByTestId(`personalization-candidate-${CREATE_ID}`);

    await userEvent.click(screen.getByTestId(`personalization-candidate-edit-${CREATE_ID}`));
    await userEvent.click(
      await screen.findByTestId(`personalization-candidate-audience-synthetic-lab-agent-${CREATE_ID}`),
    );
    await userEvent.click(screen.getByTestId(`personalization-candidate-approve-edits-${CREATE_ID}`));

    await waitFor(() => {
      expect(world.reviewPersonalizationCandidate).toHaveBeenCalledWith(
        expect.objectContaining({ audienceAgentIds: ["synthetic-lab-agent"] }),
      );
    });
  });

  it("merges with the revision the proposal was written against", async () => {
    const world = renderQueue();
    await screen.findByTestId(`personalization-candidate-${UPDATE_ID}`);

    await userEvent.click(screen.getByTestId(`personalization-candidate-merge-${UPDATE_ID}`));

    // Merging without it would fold this text over an edit made since, unseen.
    await waitFor(() => {
      expect(world.reviewPersonalizationCandidate).toHaveBeenCalledWith({
        candidateId: UPDATE_ID,
        action: "merge-into",
        mergeTargetId: "mem-0000000000000002",
        mergeExpectedRevision: 2,
      });
    });
  });

  it("offers no merge for a proposal with nothing to merge into", async () => {
    renderQueue();
    await screen.findByTestId(`personalization-candidate-${CREATE_ID}`);

    expect(screen.queryByTestId(`personalization-candidate-merge-${CREATE_ID}`)).toBeNull();
  });

  it("keeps the proposal in the queue when the target moved on", async () => {
    renderQueue({
      reviewPersonalizationCandidate: async () => {
        throw new Error("personalization-revision-conflict: expected 2, stored 7");
      },
    });
    await screen.findByTestId(`personalization-candidate-${UPDATE_ID}`);

    await userEvent.click(screen.getByTestId(`personalization-candidate-merge-${UPDATE_ID}`));

    // Refreshing the queue here would hide the candidate before the reviewer saw why nothing
    // happened.
    await waitFor(() => {
      expect(screen.getByTestId(`personalization-candidate-conflict-${UPDATE_ID}`)).toBeTruthy();
    });
    expect(screen.getByTestId(`personalization-candidate-${UPDATE_ID}`)).toBeTruthy();
  });

  it("says a decision was not recorded rather than looking as though it was", async () => {
    renderQueue({
      reviewPersonalizationCandidate: async () => {
        throw new Error("personalization-storage-unavailable");
      },
    });
    await screen.findByTestId(`personalization-candidate-${CREATE_ID}`);

    await userEvent.click(screen.getByTestId(`personalization-candidate-approve-${CREATE_ID}`));

    await waitFor(() => {
      expect(screen.getByTestId(`personalization-candidate-failed-${CREATE_ID}`)).toBeTruthy();
    });
  });

  it("says the queue is empty rather than showing nothing at all", async () => {
    renderQueue({ listPersonalizationCandidates: async () => [] });

    expect(await screen.findByTestId("personalization-review-empty")).toBeTruthy();
  });
});
