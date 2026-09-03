// @vitest-environment jsdom

import { renderHook, waitFor } from "@testing-library/react";
import { afterEach, describe, expect, it, vi } from "vitest";
import { activateAppLanguage } from "../i18n";
import { agentService } from "../services/runtime-agent-client";
import type { CodeReview } from "../types/code-review";
import type { MissionControlRunSummary } from "../types/mission-control";
import { useMissionControlReview } from "./use-mission-control-review";

afterEach(() => vi.restoreAllMocks());

const NO_REVIEW_MESSAGE = "test-no-review-message";
const ERROR_MESSAGE = "test-error-message";

function renderReview(target: MissionControlRunSummary) {
  return renderHook(() => useMissionControlReview(target, NO_REVIEW_MESSAGE, ERROR_MESSAGE));
}

function run(overrides: Partial<MissionControlRunSummary> = {}): MissionControlRunSummary {
  return {
    runId: "run-1", version: 1, ownerType: "agent", ownerId: "owner-1", agentId: "claude-code",
    title: "Run 1", state: "running", createdAt: "2026-08-16T00:00:00.000Z", updatedAt: "2026-08-16T00:00:00.000Z",
    endedAt: "2026-08-16T00:10:00.000Z", projectId: null, workspace: null, phase: null, attention: null,
    reasonCode: null, verification: "unavailable", tokens: null, cost: null, actions: [],
    navigation: { kind: "review", id: "review-1", sessionId: "session-1" },
    runner: null,
    ...overrides,
  };
}

function review(overrides: Partial<CodeReview> = {}): CodeReview {
  return {
    id: "review-1", sessionId: "session-1", workspaceId: "workspace-1", fingerprint: "fp-1",
    status: "active", decision: "pending", createdAt: "2026-08-16T00:00:00.000Z", updatedAt: "2026-08-16T00:05:00.000Z",
    files: [], comments: [], findings: [],
    summary: { changedFiles: 3, viewedFiles: 1, unresolvedComments: 2, unresolvedFindings: 0 },
    hunkDecisions: [],
    ...overrides,
  };
}

describe("useMissionControlReview", () => {
  it("starts loading with no data before the fetch settles", async () => {
    await activateAppLanguage("en");
    vi.spyOn(agentService, "getCodeReview").mockResolvedValue(review());

    const { result } = renderReview(run());

    expect(result.current.initialLoading).toBe(true);
    expect(result.current.data).toBeUndefined();
  });

  it("fetches the review by the run's own navigation.id, a single bounded record", async () => {
    await activateAppLanguage("en");
    const getReviewSpy = vi.spyOn(agentService, "getCodeReview").mockResolvedValue(review());

    const { result } = renderReview(run());

    await waitFor(() => expect(result.current.data).toBeDefined());
    expect(getReviewSpy).toHaveBeenCalledWith("review-1");
    expect(getReviewSpy).toHaveBeenCalledTimes(1);
    expect(result.current.data?.summary.changedFiles).toBe(3);
    expect(result.current.error).toBeUndefined();
  });

  it("resolves to an unavailable state, without fetching, when the run's navigation is not review-shaped", async () => {
    await activateAppLanguage("en");
    const getReviewSpy = vi.spyOn(agentService, "getCodeReview");

    const { result } = renderReview(run({ navigation: { kind: "session", id: "session-1", sessionId: null } }));

    await waitFor(() => expect(result.current.error?.kind).toBe("unavailable"));
    expect(result.current.error?.message).toBe(NO_REVIEW_MESSAGE);
    expect(getReviewSpy).not.toHaveBeenCalled();
  });

  it("resolves to an unavailable state, without fetching, when the run has no navigation at all", async () => {
    await activateAppLanguage("en");
    const getReviewSpy = vi.spyOn(agentService, "getCodeReview");

    const { result } = renderReview(run({ navigation: null }));

    await waitFor(() => expect(result.current.error?.kind).toBe("unavailable"));
    expect(getReviewSpy).not.toHaveBeenCalled();
  });

  it("resolves to a retryable error, without leaking the raw reason, when the review cannot be loaded", async () => {
    await activateAppLanguage("en");
    vi.spyOn(agentService, "getCodeReview").mockRejectedValue(new Error("token=secret"));

    const { result } = renderReview(run());

    await waitFor(() => expect(result.current.error?.kind).toBe("error"));
    expect(result.current.error?.retryable).toBe(true);
    expect(result.current.error?.message).toBe(ERROR_MESSAGE);
  });

  it("reload() re-runs the fetch, for AsyncBoundary's own retry affordance", async () => {
    await activateAppLanguage("en");
    const getReviewSpy = vi.spyOn(agentService, "getCodeReview")
      .mockRejectedValueOnce(new Error("boom"))
      .mockResolvedValueOnce(review());

    const { result } = renderReview(run());
    await waitFor(() => expect(result.current.error?.kind).toBe("error"));

    result.current.reload();

    await waitFor(() => expect(result.current.data).toBeDefined());
    expect(result.current.error).toBeUndefined();
    expect(getReviewSpy).toHaveBeenCalledTimes(2);
  });
});
