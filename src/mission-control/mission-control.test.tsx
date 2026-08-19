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
import type { MissionControlOverview } from "../types/mission-control";
import { MissionControl } from "./mission-control";

afterEach(() => { cleanup(); vi.restoreAllMocks(); resetWebMissionControlRunsForTest(); });

describe("MissionControl", () => {
  it("renders a bounded page from 1,000 Web Runs and loads detail only on inspection", async () => {
    await i18n.changeLanguage("en");
    seedWebMissionControlRunsForTest(1_000);
    const detail = vi.spyOn(webAgentClient, "getMissionControlRun");

    render(<MissionControl />);

    await waitFor(() => expect(document.querySelectorAll("[data-testid^='mission-run-']")).toHaveLength(60));
    expect(detail).not.toHaveBeenCalled();
    fireEvent.click(document.querySelector("[data-testid^='mission-run-'] button")!);
    await waitFor(() => expect(detail).toHaveBeenCalledOnce());
    // The spy records at click time, synchronously, while the tablist only exists after the
    // awaited detail resolves and React commits. Asserting it synchronously here races that
    // commit and fails under full-suite load; await the element itself, as the sibling test does.
    await waitFor(() => expect(document.querySelector("[role='tablist']")).toBeTruthy());
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
    expect(navigate).toHaveBeenCalledWith(expect.objectContaining({ kind: "review" }));
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
});
