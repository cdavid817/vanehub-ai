// @vitest-environment jsdom

import { cleanup, fireEvent, screen, waitFor } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { activateAppLanguage } from "../i18n";
import { agentService } from "../services/runtime-agent-client";
import { mockAgents } from "../services/mock-agent-data";
import { renderWithAppProviders } from "../test/render";
import type { AutomationsView } from "../main-layout/workspace-route";
import { Automations } from "./automations";

const lazyTimeout = { timeout: 15_000 };

function Harness({ initial }: { initial: AutomationsView }) {
  return <Controlled initial={initial} />;
}

import { useState } from "react";
function Controlled({ initial }: { initial: AutomationsView }) {
  const [view, setView] = useState<AutomationsView>(initial);
  return <Automations agents={mockAgents} onViewChange={setView} view={view} />;
}

let listScheduledTasks: ReturnType<typeof vi.spyOn>;
beforeEach(async () => {
  await activateAppLanguage("zh-CN");
  listScheduledTasks = vi.spyOn(agentService, "listScheduledTasks").mockResolvedValue([]);
});
afterEach(() => { cleanup(); vi.restoreAllMocks(); });

describe("Automations", () => {
  it("renders the three tabs and deep-links to the requested one without mounting the others", async () => {
    renderWithAppProviders(<Harness initial="goals" />);

    const tabs = screen.getAllByRole("tab");
    expect(tabs.map((tab) => tab.textContent)).toEqual(["循环工程", "定时任务", "目标中心"]);
    expect(screen.getByRole("tab", { name: "目标中心" }).getAttribute("aria-selected")).toBe("true");
    await waitFor(() => expect(document.querySelector("#goal-center")).toBeTruthy(), lazyTimeout);
    expect(document.querySelector("#loop-center")?.childElementCount).toBe(0);
    expect(document.querySelector("#automations-scheduled")?.childElementCount).toBe(0);
  }, 20_000);

  it("lazy-loads a tab on first visit, keeps it mounted, and preserves an unsent draft", async () => {
    renderWithAppProviders(<Harness initial="loops" />);
    await waitFor(() => expect(document.querySelector("#loop-center")?.childElementCount).toBeGreaterThan(0), lazyTimeout);

    fireEvent.click(screen.getByRole("tab", { name: "定时任务" }));
    const name = await screen.findByLabelText("任务名称", {}, lazyTimeout);
    await waitFor(() => expect(document.activeElement).toBe(name));
    fireEvent.change(name, { target: { value: "草稿任务" } });
    expect(screen.getByRole("tab", { name: "定时任务" }).getAttribute("aria-selected")).toBe("true");
    expect(document.querySelector("#loop-center")?.hasAttribute("hidden")).toBe(true);

    fireEvent.click(screen.getByRole("tab", { name: "循环工程" }));
    expect(document.querySelector("#automations-scheduled")?.hasAttribute("hidden")).toBe(true);
    expect(document.querySelector("#loop-center")?.hasAttribute("hidden")).toBe(false);
    // Hidden, not unmounted: the draft is still there when the tab comes back.
    fireEvent.click(screen.getByRole("tab", { name: "定时任务" }));
    expect((screen.getByLabelText("任务名称") as HTMLInputElement).value).toBe("草稿任务");
    // Returning re-reads the list (the scheduler kept running) without touching the draft.
    await waitFor(() => expect(listScheduledTasks).toHaveBeenCalledTimes(2));
  }, 30_000);

  it("moves selection and focus between tabs with the arrow keys, even once the Scheduled surface mounts", async () => {
    renderWithAppProviders(<Harness initial="loops" />);
    screen.getByRole("tab", { name: "循环工程" }).focus();
    fireEvent.keyDown(screen.getByRole("tab", { name: "循环工程" }), { key: "ArrowRight" });
    expect(screen.getByRole("tab", { name: "定时任务" }).getAttribute("aria-selected")).toBe("true");
    // Roving focus: the newly selected tab is the one that now has focus, and it stays there after
    // the lazily loaded surface mounts instead of being pulled into the first input.
    expect(document.activeElement).toBe(screen.getByRole("tab", { name: "定时任务" }));
    await screen.findByLabelText("任务名称", {}, lazyTimeout);
    await new Promise((resolve) => setTimeout(resolve, 20));
    expect(document.activeElement).toBe(screen.getByRole("tab", { name: "定时任务" }));
    fireEvent.keyDown(screen.getByRole("tab", { name: "定时任务" }), { key: "ArrowRight" });
    expect(screen.getByRole("tab", { name: "目标中心" }).getAttribute("aria-selected")).toBe("true");
    expect(document.activeElement).toBe(screen.getByRole("tab", { name: "目标中心" }));
    fireEvent.keyDown(screen.getByRole("tab", { name: "目标中心" }), { key: "ArrowLeft" });
    expect(screen.getByRole("tab", { name: "定时任务" }).getAttribute("aria-selected")).toBe("true");
    // A click on the tab is the path that hands focus to the surface's first control.
    fireEvent.click(screen.getByRole("tab", { name: "定时任务" }));
    await waitFor(() => expect(document.activeElement).toBe(screen.getByLabelText("任务名称")));
  }, 20_000);

  it("does not visit the placeholder tab a hidden shell is handed", async () => {
    const definitions = vi.spyOn(agentService, "listLoopDefinitions");
    const { rerender } = renderWithAppProviders(<Automations agents={mockAgents} onViewChange={vi.fn()} view="scheduled" />);
    await screen.findByLabelText("任务名称", {}, lazyTimeout);
    // Leaving Automations: the shell keeps the last tab, but even a different placeholder must
    // not mount a surface while nothing is on screen.
    rerender(<Automations active={false} agents={mockAgents} onViewChange={vi.fn()} view="loops" />);
    await new Promise((resolve) => setTimeout(resolve, 50));
    expect(document.querySelector("#loop-center")?.childElementCount).toBe(0);
    expect(definitions).not.toHaveBeenCalled();
  }, 20_000);

  it("does not pull focus into the tablist when the view changes from outside", async () => {
    const { rerender } = renderWithAppProviders(<Automations agents={mockAgents} onViewChange={vi.fn()} view="loops" />);
    document.body.focus();
    rerender(<Automations agents={mockAgents} onViewChange={vi.fn()} view="goals" />);
    expect(screen.getByRole("tab", { name: "目标中心" }).getAttribute("aria-selected")).toBe("true");
    expect(document.activeElement).toBe(document.body);
  });
});
