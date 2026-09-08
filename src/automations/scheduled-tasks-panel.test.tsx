// @vitest-environment jsdom

import { cleanup, render, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { afterEach, describe, expect, it, vi } from "vitest";
import { i18n } from "../i18n";
import { agentService } from "../services/runtime-agent-client";
import { mockAgents } from "../services/mock-agent-data";
import type { ScheduledTask } from "../types/agent";
import { ScheduledTasksPanel } from "./scheduled-tasks-panel";

afterEach(() => { cleanup(); vi.restoreAllMocks(); });

describe("ScheduledTasksPanel", () => {
  it("renders inline without a dialog and focuses the first control of the surface", async () => {
    await i18n.changeLanguage("zh-CN");
    vi.spyOn(agentService, "listScheduledTasks").mockResolvedValue([]);
    render(<ScheduledTasksPanel agents={mockAgents} />);

    expect(await screen.findByRole("heading", { name: "定时任务" })).toBeTruthy();
    expect(screen.queryByRole("dialog")).toBeNull();
    expect(document.querySelector("[aria-modal]")).toBeNull();
    expect(document.activeElement).toBe(screen.getByLabelText("任务名称"));
    expect(screen.getByPlaceholderText("例如：每日整理项目进度")).toBeTruthy();
  });

  it("validates and creates a localized scheduled task for a stable Agent id", async () => {
    await i18n.changeLanguage("zh-CN");
    const user = userEvent.setup();
    const created = taskFixture();
    vi.spyOn(agentService, "listScheduledTasks").mockResolvedValue([]);
    const create = vi.spyOn(agentService, "createScheduledTask").mockResolvedValue(created);
    render(<ScheduledTasksPanel agents={mockAgents} />);

    const createButton = await screen.findByRole("button", { name: "创建任务" });
    expect((createButton as HTMLButtonElement).disabled).toBe(true);
    await user.type(screen.getByLabelText("任务名称"), " 每周代码检查 ");
    await user.type(screen.getByLabelText("任务内容"), " 运行仓库健康检查 ");
    await user.selectOptions(screen.getByLabelText("Agent 工具"), "opencode");
    await user.selectOptions(screen.getByLabelText("执行频率"), "weekly");
    await user.selectOptions(screen.getByLabelText("星期"), "1");
    await user.clear(screen.getByLabelText("执行时间"));
    await user.type(screen.getByLabelText("执行时间"), "09:30");
    await user.click(createButton);

    await waitFor(() => expect(create).toHaveBeenCalledWith({
      agentId: "opencode",
      content: "运行仓库健康检查",
      frequency: { kind: "weekly", weekday: 1, timeOfDay: "09:30" },
      name: "每周代码检查",
    }));
    expect(await screen.findByText("每周周一 09:30")).toBeTruthy();
    expect(screen.getByText("每周仓库检查")).toBeTruthy();
  });

  it("blocks invalid recurrence values with accessible feedback", async () => {
    await i18n.changeLanguage("zh-CN");
    const user = userEvent.setup();
    vi.spyOn(agentService, "listScheduledTasks").mockResolvedValue([]);
    const create = vi.spyOn(agentService, "createScheduledTask");
    render(<ScheduledTasksPanel agents={mockAgents} />);

    await user.type(await screen.findByLabelText("任务名称"), "检查");
    await user.type(screen.getByLabelText("任务内容"), "检查项目");
    await user.selectOptions(screen.getByLabelText("执行频率"), "minutes");
    await user.clear(screen.getByLabelText("间隔"));

    expect(screen.getByRole("alert").textContent).toContain("请输入有效的执行频率");
    expect((screen.getByRole("button", { name: "创建任务" }) as HTMLButtonElement).disabled).toBe(true);
    expect(create).not.toHaveBeenCalled();
  });

  it("scopes pending state to a task and applies the authoritative result", async () => {
    await i18n.changeLanguage("zh-CN");
    const user = userEvent.setup();
    const task = taskFixture();
    let resolveMutation: ((value: ScheduledTask) => void) | undefined;
    vi.spyOn(agentService, "listScheduledTasks").mockResolvedValue([task]);
    vi.spyOn(agentService, "setScheduledTaskEnabled").mockReturnValue(new Promise((resolve) => { resolveMutation = resolve; }));
    render(<ScheduledTasksPanel agents={mockAgents} />);

    const toggle = await screen.findByRole("switch", { name: "停用任务“每周代码检查”" });
    await user.click(toggle);
    expect((toggle as HTMLButtonElement).disabled).toBe(true);
    expect(screen.getByText("每周仓库检查")).toBeTruthy();

    resolveMutation?.({ ...task, enabled: false });
    expect(await screen.findByRole("switch", { name: "启用任务“每周代码检查”" })).toBeTruthy();
  });

  it("re-reads the list on every activation while keeping the draft and the visible rows", async () => {
    await i18n.changeLanguage("zh-CN");
    const user = userEvent.setup();
    const task = taskFixture();
    const list = vi.spyOn(agentService, "listScheduledTasks")
      .mockResolvedValueOnce([task])
      .mockResolvedValueOnce([{ ...task, latestStatus: "failed", latestError: "runner exited" }])
      .mockRejectedValueOnce(new Error("native busy"));
    const { rerender } = render(<ScheduledTasksPanel agents={mockAgents} />);
    await screen.findByText("每周仓库检查");
    await user.type(screen.getByLabelText("任务名称"), "草稿");

    rerender(<ScheduledTasksPanel active={false} agents={mockAgents} />);
    rerender(<ScheduledTasksPanel agents={mockAgents} />);
    // The scheduler ran while the tab was hidden; the updated status shows on return.
    await waitFor(() => expect(list).toHaveBeenCalledTimes(2));
    expect(await screen.findByText(/runner exited/)).toBeTruthy();
    expect((screen.getByLabelText("任务名称") as HTMLInputElement).value).toBe("草稿");

    // A failed refresh keeps the rows that were already on screen and surfaces the error.
    rerender(<ScheduledTasksPanel active={false} agents={mockAgents} />);
    rerender(<ScheduledTasksPanel agents={mockAgents} />);
    await waitFor(() => expect(list).toHaveBeenCalledTimes(3));
    expect((await screen.findByRole("alert")).textContent).toContain("native busy");
    expect(screen.getByText("每周仓库检查")).toBeTruthy();
  });

  it("does not take focus when told the activation came from the keyboard", async () => {
    await i18n.changeLanguage("zh-CN");
    vi.spyOn(agentService, "listScheduledTasks").mockResolvedValue([]);
    render(<ScheduledTasksPanel agents={mockAgents} focusOnActivate={false} />);
    await screen.findByRole("heading", { name: "定时任务" });
    expect(document.activeElement).toBe(document.body);
  });

  it("keeps a failed load's error and the existing list visible after a failed mutation", async () => {
    await i18n.changeLanguage("zh-CN");
    const user = userEvent.setup();
    const task = taskFixture();
    vi.spyOn(agentService, "listScheduledTasks").mockResolvedValue([task]);
    vi.spyOn(agentService, "setScheduledTaskEnabled").mockRejectedValue(new Error("native refused"));
    render(<ScheduledTasksPanel agents={mockAgents} />);

    await user.click(await screen.findByRole("switch", { name: "停用任务“每周代码检查”" }));
    expect((await screen.findByRole("alert")).textContent).toContain("native refused");
    expect(screen.getByText("每周仓库检查")).toBeTruthy();
    expect(screen.getByRole("switch", { name: "停用任务“每周代码检查”" })).toBeTruthy();
  });
});

function taskFixture(): ScheduledTask {
  return {
    id: "scheduled-task-1",
    name: "每周代码检查",
    content: "每周仓库检查",
    agentId: "opencode",
    frequency: { kind: "weekly", weekday: 1, timeOfDay: "09:30" },
    enabled: true,
    nextRunAt: "2026-08-31T01:30:00.000Z",
    latestStatus: "never-run",
    latestRunAt: null,
    latestRunSessionId: null,
    latestError: null,
    createdAt: "2026-08-25T01:00:00.000Z",
    updatedAt: "2026-08-25T01:00:00.000Z",
  };
}
