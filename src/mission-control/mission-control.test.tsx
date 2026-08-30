// @vitest-environment jsdom

import { cleanup, fireEvent, render, screen, waitFor } from "@testing-library/react";
import { afterEach, describe, expect, it, vi } from "vitest";
import { i18n } from "../i18n";
import { agentService } from "../services/runtime-agent-client";
import {
  resetWebMissionControlRunsForTest,
  seedWebMissionControlRunsForTest,
  webAgentClient,
} from "../services/web-agent-client";
import type { MissionControlOverview, MissionControlRunDetail } from "../types/mission-control";
import { MissionControl } from "./mission-control";

// `sessionStorage.clear()`: mission-control-view-state.ts persists filters there (4.8), and
// jsdom's storage is one shared global across every test in this file — without clearing it, a
// filter set by an earlier test silently becomes a later test's initial state.
afterEach(() => { cleanup(); vi.restoreAllMocks(); resetWebMissionControlRunsForTest(); sessionStorage.clear(); });

describe("MissionControl", () => {
  it("renders a bounded page from 1,000 Web Runs and loads detail only on inspection", async () => {
    await i18n.changeLanguage("en");
    seedWebMissionControlRunsForTest(1_000);
    const detail = vi.spyOn(webAgentClient, "getMissionControlRun");

    render(<MissionControl />);

    await waitFor(() => expect(document.querySelectorAll("[data-testid^='mission-run-']")).toHaveLength(60));
    expect(detail).not.toHaveBeenCalled();
    fireEvent.click(document.querySelector("[data-testid^='mission-run-'] button")!);
    // The tablist is what proves detail actually rendered, so it belongs inside the waitFor.
    // Asserting it synchronously after only awaiting the spy call raced: the spy fires when the
    // request is *issued*, leaving promise resolution and React's commit still pending — which a
    // loaded CI runner does not finish within the same tick. Verified by injecting a 50ms delay
    // into the detail call: this shape passes, the synchronous one fails on exactly this line.
    await waitFor(() => {
      expect(detail).toHaveBeenCalledOnce();
      expect(document.querySelector("[role='tablist']")).toBeTruthy();
    });
  });

  it("prioritizes attention, freezes terminal elapsed time, filters, inspects, and navigates", async () => {
    await i18n.changeLanguage("en"); const navigate = vi.fn();
    const overview = vi.spyOn(agentService, "getMissionControlOverview");
    render(<MissionControl onNavigate={navigate} />);
    await waitFor(() => expect(document.querySelectorAll("[data-testid^='mission-run-']").length).toBeGreaterThan(0));
    expect(document.querySelector("[data-runner='ssh']")?.textContent).toContain("build.example.test");
    fireEvent.change(screen.getByLabelText(/Runner/), { target: { value: "ssh" } });
    await waitFor(() => expect(overview).toHaveBeenLastCalledWith(expect.objectContaining({ runner: "ssh", cursor: null })));
    const statusFilter = document.querySelector("select") as HTMLSelectElement;
    fireEvent.change(statusFilter, { target: { value: "failed" } });
    const failed = await screen.findAllByTestId("mission-run-018f0f17-4d6a-7e20-b41d-66c5271a294");
    fireEvent.click(failed[0].querySelector("button")!);
    await waitFor(() => expect(document.querySelector("[role='tablist']")).toBeTruthy());
    fireEvent.click(document.querySelector("[data-action='review']")!);
    expect(navigate).toHaveBeenCalledWith(
      expect.objectContaining({ kind: "review" }),
      "018f0f17-4d6a-7e20-b41d-66c5271a294",
    );
  });

  it("does not let a slower inspect() response overwrite a more recently selected run's detail", async () => {
    await i18n.changeLanguage("en");
    seedWebMissionControlRunsForTest(100);
    render(<MissionControl />);
    await waitFor(() => expect(document.querySelectorAll("[data-testid^='mission-run-']").length).toBeGreaterThanOrEqual(2));

    const rows = Array.from(document.querySelectorAll("[data-testid^='mission-run-']"));
    const firstRunId = rows[0].getAttribute("data-testid")!.replace("mission-run-", "");
    const secondRunId = rows[1].getAttribute("data-testid")!.replace("mission-run-", "");
    // Real, valid detail fixtures fetched through the underlying client, before the spy below
    // starts intercepting the very same method on `agentService`.
    const firstDetail = await webAgentClient.getMissionControlRun(firstRunId);
    const secondDetail = await webAgentClient.getMissionControlRun(secondRunId);

    let resolveFirst: ((detail: MissionControlRunDetail) => void) | undefined;
    let resolveSecond: ((detail: MissionControlRunDetail) => void) | undefined;
    vi.spyOn(agentService, "getMissionControlRun")
      .mockImplementationOnce(() => new Promise((resolve) => { resolveFirst = resolve; }))
      .mockImplementationOnce(() => new Promise((resolve) => { resolveSecond = resolve; }));

    fireEvent.click(rows[0].querySelector("button")!);
    fireEvent.click(rows[1].querySelector("button")!);

    // The second (more recent) click's response arrives first, as it normally would.
    resolveSecond?.(secondDetail);
    await waitFor(() => expect(document.querySelector("aside [role='tablist']")).toBeTruthy());
    expect(document.querySelector("aside")!.querySelector(`[data-testid='mission-run-${secondRunId}']`)).toBeTruthy();

    // The stale first click's response arrives late — must not clobber the second run's detail.
    resolveFirst?.(firstDetail);
    await new Promise((resolve) => setTimeout(resolve, 0));
    expect(document.querySelector("aside")!.querySelector(`[data-testid='mission-run-${secondRunId}']`)).toBeTruthy();
    expect(document.querySelector("aside")!.querySelector(`[data-testid='mission-run-${firstRunId}']`)).toBeNull();
  });

  it("shows safe errors and does not expose backend diagnostics", async () => {
    await i18n.changeLanguage("en");
    vi.spyOn(agentService, "getMissionControlOverview").mockRejectedValue(new Error("token=secret"));
    render(<MissionControl />);
    await waitFor(() => expect(document.querySelector("[aria-live='polite']")).toBeTruthy());
    expect(document.body.textContent).not.toContain("secret");
  });

  it("keeps all Mission Control resources aligned across registered locales", () => {
    for (const locale of ["en", "zh-CN", "zh-TW", "ja", "ko"]) {
      for (const key of ["title", "attention", "sortAttention", "action.review", "facet.logs", "state.waiting_approval"]) {
        expect(i18n.getFixedT(locale)(`missionControl.${key}`)).not.toBe(`missionControl.${key}`);
      }
    }
  });

  it("renders an explicit empty state", async () => {
    const empty: MissionControlOverview = { counts: { running: 0, waitingApproval: 0, waitingUser: 0, retrying: 0, blocked: 0, failed: 0, completedRecently: 0 }, attention: { items: [], nextCursor: null }, active: { items: [], nextCursor: null }, recent: { items: [], nextCursor: null } };
    vi.spyOn(agentService, "getMissionControlOverview").mockResolvedValue(empty);
    render(<MissionControl />); await waitFor(() => expect(document.querySelector("[data-testid='mission-control'] .p-8")).toBeTruthy());
  });

  it("stops polling once unmounted", async () => {
    const empty: MissionControlOverview = { counts: { running: 0, waitingApproval: 0, waitingUser: 0, retrying: 0, blocked: 0, failed: 0, completedRecently: 0 }, attention: { items: [], nextCursor: null }, active: { items: [], nextCursor: null }, recent: { items: [], nextCursor: null } };
    const overview = vi.spyOn(agentService, "getMissionControlOverview").mockResolvedValue(empty);
    const { unmount } = render(<MissionControl />);
    await waitFor(() => expect(overview).toHaveBeenCalled());

    unmount();
    const callsAtUnmount = overview.mock.calls.length;
    // Longer than the 2s poll interval: proves the interval, not just this test's patience, stopped.
    await new Promise((resolve) => setTimeout(resolve, 2_500));
    expect(overview.mock.calls.length).toBe(callsAtUnmount);
  });
});
