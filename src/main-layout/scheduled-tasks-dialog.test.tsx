// @vitest-environment jsdom

import { render, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { afterEach, describe, expect, it, vi } from "vitest";
import { i18n } from "../i18n";
import { agentService } from "../services/runtime-agent-client";
import { mockAgents } from "../services/mock-agent-data";
import type { ScheduledTask } from "../types/agent";
import { ScheduledTasksDialog } from "./scheduled-tasks-dialog";

afterEach(() => vi.restoreAllMocks());

describe("ScheduledTasksDialog", () => {
  it("validates and creates a localized scheduled task for a stable Agent id", async () => {
    await i18n.changeLanguage("zh-CN");
    const user = userEvent.setup();
    const created = taskFixture();
    vi.spyOn(agentService, "listScheduledTasks").mockResolvedValue([]);
    const create = vi.spyOn(agentService, "createScheduledTask").mockResolvedValue(created);
    render(<ScheduledTasksDialog agents={mockAgents} onClose={vi.fn()} open />);

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
    render(<ScheduledTasksDialog agents={mockAgents} onClose={vi.fn()} open />);

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
    render(<ScheduledTasksDialog agents={mockAgents} onClose={vi.fn()} open />);

    const toggle = await screen.findByRole("switch", { name: "停用任务“每周代码检查”" });
    await user.click(toggle);
    expect((toggle as HTMLButtonElement).disabled).toBe(true);
    expect(screen.getByText("每周仓库检查")).toBeTruthy();

    resolveMutation?.({ ...task, enabled: false });
    expect(await screen.findByRole("switch", { name: "启用任务“每周代码检查”" })).toBeTruthy();
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
