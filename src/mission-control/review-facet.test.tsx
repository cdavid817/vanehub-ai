// @vitest-environment jsdom

import { cleanup, render, screen, waitFor } from "@testing-library/react";
import { afterEach, describe, expect, it, vi } from "vitest";
import { activateAppLanguage, i18n } from "../i18n";
import { agentService } from "../services/runtime-agent-client";
import type { CodeReview } from "../types/code-review";
import type { MissionControlRunSummary } from "../types/mission-control";
import { ReviewFacet } from "./review-facet";

afterEach(() => { cleanup(); vi.restoreAllMocks(); });

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

describe("ReviewFacet", () => {
  it("renders the review's own status, decision, and bounded summary counts", async () => {
    await activateAppLanguage("en");
    vi.spyOn(agentService, "getCodeReview").mockResolvedValue(review({
      status: "completed", decision: "changes-requested",
      summary: { changedFiles: 5, viewedFiles: 2, unresolvedComments: 1, unresolvedFindings: 4 },
    }));

    render(<ReviewFacet run={run()} />);

    await waitFor(() => expect(screen.getByText("Completed")).toBeTruthy());
    expect(screen.getByText("Changes requested")).toBeTruthy();
    expect(screen.getByText("5")).toBeTruthy();
    expect(screen.getByText("4")).toBeTruthy();
    // A bounded summary, not the full review surface — no comment/finding bodies, no diff content.
    expect(screen.queryByRole("region", { name: /diff/i })).toBeNull();
  });

  it("renders the no-review state, without fetching, when the run's navigation is not review-shaped", async () => {
    await activateAppLanguage("en");
    const getReviewSpy = vi.spyOn(agentService, "getCodeReview");

    render(<ReviewFacet run={run({ navigation: { kind: "session", id: "session-1", sessionId: null } })} />);

    await waitFor(() => expect(screen.getByText("No review is linked to this Run.")).toBeTruthy());
    expect(getReviewSpy).not.toHaveBeenCalled();
  });

  it("shows a safe error and does not leak backend diagnostics when the review cannot be loaded", async () => {
    await activateAppLanguage("en");
    vi.spyOn(agentService, "getCodeReview").mockRejectedValue(new Error("token=secret"));

    render(<ReviewFacet run={run()} />);

    await waitFor(() => expect(screen.getByText("Could not load the review for this Run.")).toBeTruthy());
    expect(document.body.textContent).not.toContain("secret");
  });

  it("loads and translates every locale's new review-facet strings, not falling back to zh-CN", async () => {
    for (const locale of ["en", "zh-CN", "zh-TW", "ja", "ko"] as const) {
      await activateAppLanguage(locale);
      expect(i18n.hasResourceBundle(locale, "translation")).toBe(true);
      const t = i18n.getFixedT(locale);
      for (const key of [
        "noReview", "error", "status.active", "status.completed",
        "decision.pending", "decision.accepted", "decision.changes-requested",
        "changedFiles", "viewedFiles", "unresolvedComments", "unresolvedFindings",
      ]) {
        expect(t(`missionControl.review.${key}`)).not.toBe(`missionControl.review.${key}`);
      }
    }
  });
});
