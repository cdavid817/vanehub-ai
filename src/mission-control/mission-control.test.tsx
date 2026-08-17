// @vitest-environment jsdom

import { cleanup, fireEvent, render, screen, waitFor } from "@testing-library/react";
import { afterEach, describe, expect, it, vi } from "vitest";
import { i18n } from "../i18n";
import { agentService } from "../services/runtime-agent-client";
import type { MissionControlOverview } from "../types/mission-control";
import { MissionControl } from "./mission-control";

afterEach(() => { cleanup(); vi.restoreAllMocks(); });

describe("MissionControl", () => {
  it("prioritizes attention, freezes terminal elapsed time, filters, inspects, and navigates", async () => {
    await i18n.changeLanguage("en"); const navigate = vi.fn();
    render(<MissionControl onNavigate={navigate} />);
    await waitFor(() => expect(document.querySelectorAll("[data-testid^='mission-run-']").length).toBeGreaterThan(0));
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
