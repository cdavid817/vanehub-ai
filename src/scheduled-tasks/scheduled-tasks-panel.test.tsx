// @vitest-environment jsdom

import { fireEvent, render, screen, waitFor, within } from "@testing-library/react";
import { beforeAll, beforeEach, describe, expect, it, vi } from "vitest";
import { activateAppLanguage, i18n } from "../i18n";
import type { AgentRegistryEntry, ScheduledTask, ScheduledTaskFrequency } from "../types/agent";

const mocks = vi.hoisted(() => ({
  createScheduledTask: vi.fn(),
  deleteScheduledTask: vi.fn(),
  listScheduledTaskRuns: vi.fn(),
  listScheduledTasks: vi.fn(),
  runScheduledTaskNow: vi.fn(),
  setScheduledTaskEnabled: vi.fn(),
  updateScheduledTask: vi.fn(),
}));

vi.mock("../services/runtime-agent-client", () => ({
  agentService: {
    listScheduledTasks: mocks.listScheduledTasks,
    listScheduledTaskRuns: mocks.listScheduledTaskRuns,
    createScheduledTask: mocks.createScheduledTask,
    setScheduledTaskEnabled: mocks.setScheduledTaskEnabled,
    updateScheduledTask: mocks.updateScheduledTask,
    deleteScheduledTask: mocks.deleteScheduledTask,
    runScheduledTaskNow: mocks.runScheduledTaskNow,
  },
}));

import { ScheduledTasksPanel } from "./scheduled-tasks-panel";

function buildTask(id: string, overrides: Partial<ScheduledTask> = {}): ScheduledTask {
  return {
    id,
    name: `Task ${id}`,
    content: "Do the thing",
    agentId: "onepiece",
    frequency: { kind: "daily", timeOfDay: "09:00" } as ScheduledTaskFrequency,
    enabled: true,
    nextRunAt: "2026-08-31T09:00:00.000Z",
    latestStatus: "never-run",
    latestRunAt: null,
    latestRunSessionId: null,
    latestError: null,
    createdAt: "2026-08-01T00:00:00.000Z",
    updatedAt: "2026-08-01T00:00:00.000Z",
    version: 1,
    ...overrides,
  };
}

const agents: AgentRegistryEntry[] = [
  { id: "onepiece", displayName: "OnePiece", supportedInteractionModes: ["cli"] } as AgentRegistryEntry,
];

function openMoreMenu(taskId: string) {
  const row = screen.getByTestId(`scheduled-task-select-${taskId}`).closest("li");
  if (!row) throw new Error(`row for ${taskId} not found`);
  fireEvent.click(within(row).getByRole("button", { name: "More actions" }));
  return row;
}

describe("ScheduledTasksPanel", () => {
  beforeAll(async () => activateAppLanguage("en"));

  beforeEach(() => {
    for (const mock of Object.values(mocks)) mock.mockReset();
    mocks.listScheduledTasks.mockResolvedValue([]);
    mocks.listScheduledTaskRuns.mockResolvedValue([]);
  });

  it("loads the task list on mount without needing a dialog `open` prop, and offers a New task trigger", async () => {
    render(<ScheduledTasksPanel agents={agents} />);
    expect(await screen.findByText("No scheduled tasks yet.")).toBeTruthy();
    expect(screen.getByRole("button", { name: "New task" })).toBeTruthy();
  });

  it("renders the frequency summary through i18next interpolation instead of a hard-coded string", async () => {
    mocks.listScheduledTasks.mockResolvedValueOnce([
      buildTask("t-weekly", { frequency: { kind: "weekly", weekday: 1, timeOfDay: "09:00" } }),
      buildTask("t-minute", { frequency: { kind: "minutes", interval: 1 } }),
    ]);
    render(<ScheduledTasksPanel agents={agents} />);
    expect(await screen.findByText("Weekly Mon at 09:00")).toBeTruthy();
    expect(screen.getByText("Every 1 minute")).toBeTruthy();
  });

  it("shows locale-native weekday names in the New task sheet's weekday select instead of hard-coded English", async () => {
    await activateAppLanguage("ja");
    render(<ScheduledTasksPanel agents={agents} />);
    await screen.findByText(i18n.t("scheduledTasks.empty"));

    fireEvent.click(screen.getByRole("button", { name: i18n.t("scheduledTasks.createTitle") }));
    const frequencySelect = await screen.findByLabelText(i18n.t("scheduledTasks.frequency"));
    fireEvent.change(frequencySelect, { target: { value: "weekly" } });

    const weekdaySelect = screen.getByTestId("scheduled-task-weekday") as HTMLSelectElement;
    expect(Array.from(weekdaySelect.options).map((option) => option.textContent)).toEqual([
      "日", "月", "火", "水", "木", "金", "土",
    ]);

    await activateAppLanguage("en");
  });

  it("selecting a task from the list shows it in the detail view", async () => {
    mocks.listScheduledTasks.mockResolvedValueOnce([
      buildTask("t-a", { frequency: { kind: "daily", timeOfDay: "09:00" } }),
      buildTask("t-b", { frequency: { kind: "hours", interval: 2 } }),
    ]);
    render(<ScheduledTasksPanel agents={agents} />);
    await screen.findByTestId("scheduled-task-select-t-a");

    expect(within(screen.getByTestId("scheduled-task-detail")).getByText(i18n.t("scheduledTasks.detailEmpty"))).toBeTruthy();

    fireEvent.click(screen.getByTestId("scheduled-task-select-t-a"));
    expect(within(screen.getByTestId("scheduled-task-detail")).getByText("Task t-a")).toBeTruthy();

    fireEvent.click(screen.getByTestId("scheduled-task-select-t-b"));
    expect(within(screen.getByTestId("scheduled-task-detail")).getByText("Task t-b")).toBeTruthy();
    expect(within(screen.getByTestId("scheduled-task-detail")).queryByText("Task t-a")).toBeNull();
  });

  it("uses the scheduleId prop as the initial selection, matching the route's own current task", async () => {
    mocks.listScheduledTasks.mockResolvedValueOnce([
      buildTask("t-a", { frequency: { kind: "daily", timeOfDay: "09:00" } }),
      buildTask("t-b", { frequency: { kind: "hours", interval: 2 } }),
    ]);
    render(<ScheduledTasksPanel agents={agents} scheduleId="t-b" />);
    expect(await within(await screen.findByTestId("scheduled-task-detail")).findByText("Task t-b")).toBeTruthy();
  });

  it("reports a selection change through onSelectSchedule so the route can be updated", async () => {
    mocks.listScheduledTasks.mockResolvedValueOnce([buildTask("t-a")]);
    const onSelectSchedule = vi.fn();
    render(<ScheduledTasksPanel agents={agents} onSelectSchedule={onSelectSchedule} />);
    fireEvent.click(await screen.findByTestId("scheduled-task-select-t-a"));
    expect(onSelectSchedule).toHaveBeenCalledWith("t-a");
  });

  // 19.11: selecting a task is what actually drives the history fetch -- this is the one place
  // that proves `useScheduledTaskHistory` is really wired into the panel, not just unit-tested in
  // isolation against a hand-built state prop.
  it("fetches and renders run history for the selected task", async () => {
    mocks.listScheduledTasks.mockResolvedValueOnce([buildTask("t-a")]);
    mocks.listScheduledTaskRuns.mockResolvedValueOnce([
      { id: "run-1", taskId: "t-a", sessionId: "session-1", status: "succeeded", error: null, startedAt: "2026-08-30T09:00:00.000Z", completedAt: "2026-08-30T09:05:00.000Z" },
    ]);
    render(<ScheduledTasksPanel agents={agents} />);
    fireEvent.click(await screen.findByTestId("scheduled-task-select-t-a"));

    await waitFor(() => expect(mocks.listScheduledTaskRuns).toHaveBeenCalledWith("t-a"));
    expect(await within(screen.getByTestId("scheduled-task-history")).findByText(i18n.t("scheduledTasks.history.status.succeeded"))).toBeTruthy();
  });

  // 19.7/19.9: Create and Duplicate both open the same editor sheet.
  describe("editor sheet", () => {
    it("New task creates via createScheduledTask and selects the created task", async () => {
      render(<ScheduledTasksPanel agents={agents} />);
      await screen.findByText("No scheduled tasks yet.");

      fireEvent.click(screen.getByRole("button", { name: "New task" }));
      fireEvent.change(screen.getByLabelText(i18n.t("scheduledTasks.name")), { target: { value: "Weekly report" } });
      fireEvent.change(screen.getByLabelText(i18n.t("scheduledTasks.content")), { target: { value: "Summarize the week" } });
      mocks.createScheduledTask.mockResolvedValueOnce(buildTask("t-new", { name: "Weekly report" }));

      fireEvent.click(screen.getByRole("button", { name: "Create task" }));
      await waitFor(() => expect(mocks.createScheduledTask).toHaveBeenCalledWith({
        agentId: "onepiece", content: "Summarize the week", frequency: { kind: "daily", timeOfDay: "09:00" }, name: "Weekly report",
      }));
      // The sheet closes and the newly created task becomes the selection.
      await waitFor(() => expect(screen.queryByRole("heading", { name: "New task" })).toBeNull());
      expect(within(screen.getByTestId("scheduled-task-detail")).getByText("Weekly report")).toBeTruthy();
    });

    it("Duplicate opens the same sheet in create mode, prefilled with an adjusted name, and never calls updateScheduledTask", async () => {
      mocks.listScheduledTasks.mockResolvedValueOnce([buildTask("t-a", { name: "Nightly digest" })]);
      render(<ScheduledTasksPanel agents={agents} />);
      await screen.findByTestId("scheduled-task-select-t-a");

      openMoreMenu("t-a");
      fireEvent.click(screen.getByRole("menuitem", { name: "Duplicate task" }));

      expect(screen.getByRole("heading", { name: "New task" })).toBeTruthy();
      expect((screen.getByLabelText(i18n.t("scheduledTasks.name")) as HTMLInputElement).value).toBe("Nightly digest copy");

      mocks.createScheduledTask.mockResolvedValueOnce(buildTask("t-dup", { name: "Nightly digest copy" }));
      fireEvent.click(screen.getByRole("button", { name: "Create task" }));
      await waitFor(() => expect(mocks.createScheduledTask).toHaveBeenCalledWith(expect.objectContaining({ name: "Nightly digest copy" })));
      expect(mocks.updateScheduledTask).not.toHaveBeenCalled();
    });

    it("Edit opens the sheet prefilled from the row and saves through the version-aware updateScheduledTask", async () => {
      mocks.listScheduledTasks.mockResolvedValueOnce([buildTask("t-a", { name: "Nightly digest", version: 4 })]);
      render(<ScheduledTasksPanel agents={agents} />);
      await screen.findByTestId("scheduled-task-select-t-a");

      openMoreMenu("t-a");
      fireEvent.click(screen.getByRole("menuitem", { name: "Edit task" }));
      expect(screen.getByRole("heading", { name: "Edit task Nightly digest" })).toBeTruthy();

      fireEvent.change(screen.getByLabelText(i18n.t("scheduledTasks.name")), { target: { value: "Nightly digest v2" } });
      mocks.updateScheduledTask.mockResolvedValueOnce(buildTask("t-a", { name: "Nightly digest v2", version: 5 }));
      fireEvent.click(screen.getByRole("button", { name: "Save" }));

      await waitFor(() => expect(mocks.updateScheduledTask).toHaveBeenCalledWith(expect.objectContaining({ expectedVersion: 4, name: "Nightly digest v2", taskId: "t-a" })));
      await waitFor(() => expect(screen.queryByRole("heading", { name: /Edit task/ })).toBeNull());
    });
  });

  // 19.16
  describe("Delete", () => {
    it("lives in the row's More menu, requires confirmation, and clears the selection if the deleted task was selected", async () => {
      mocks.listScheduledTasks.mockResolvedValueOnce([buildTask("t-a")]);
      mocks.deleteScheduledTask.mockResolvedValueOnce(undefined);
      render(<ScheduledTasksPanel agents={agents} />);
      fireEvent.click(await screen.findByTestId("scheduled-task-select-t-a"));
      expect(within(screen.getByTestId("scheduled-task-detail")).getByText("Task t-a")).toBeTruthy();

      openMoreMenu("t-a");
      fireEvent.click(screen.getByRole("menuitem", { name: "Delete task" }));
      fireEvent.click(await screen.findByRole("button", { name: "Confirm delete" }));

      await waitFor(() => expect(mocks.deleteScheduledTask).toHaveBeenCalledWith("t-a"));
      await waitFor(() => expect(screen.queryByTestId("scheduled-task-select-t-a")).toBeNull());
      expect(within(screen.getByTestId("scheduled-task-detail")).getByText(i18n.t("scheduledTasks.detailEmpty"))).toBeTruthy();
    });
  });

  // 19.17: target-local pending -- one row's own in-flight mutation must not disturb another
  // row's state, the list, or the current selection. This is the same claim
  // use-scheduled-tasks-actions.test.ts proves at the hook level, exercised here through real DOM
  // interaction on two full rows.
  it("keeps two different rows' own pending/selection state fully independent", async () => {
    mocks.listScheduledTasks.mockResolvedValueOnce([buildTask("t-a"), buildTask("t-b")]);
    let resolveEnable: (value: ScheduledTask) => void = () => {};
    mocks.setScheduledTaskEnabled.mockImplementation(() => new Promise((resolve) => { resolveEnable = resolve; }));
    render(<ScheduledTasksPanel agents={agents} />);
    await screen.findByTestId("scheduled-task-select-t-a");

    // Select row B, then toggle row A's own checkbox -- an unrelated row's mutation must not move
    // the current selection.
    fireEvent.click(screen.getByTestId("scheduled-task-select-t-b"));
    const rowA = screen.getByTestId("scheduled-task-select-t-a").closest("li");
    if (!rowA) throw new Error("row t-a not found");
    fireEvent.click(within(rowA).getByRole("checkbox"));

    // Row A's own checkbox disables while pending; row B's stays interactive and the selection
    // (still row B) is untouched.
    await waitFor(() => expect((within(rowA).getByRole("checkbox") as HTMLInputElement).disabled).toBe(true));
    expect(within(screen.getByTestId("scheduled-task-detail")).getByText("Task t-b")).toBeTruthy();
    const rowB = screen.getByTestId("scheduled-task-select-t-b").closest("li");
    if (!rowB) throw new Error("row t-b not found");
    expect((within(rowB).getByRole("checkbox") as HTMLInputElement).disabled).toBe(false);

    resolveEnable(buildTask("t-a", { enabled: false }));
    await waitFor(() => expect((within(rowA).getByRole("checkbox") as HTMLInputElement).disabled).toBe(false));
    // Both rows -- and the collection itself -- survive the mutation; nothing vanished or reset.
    expect(screen.getByTestId("scheduled-task-select-t-a")).toBeTruthy();
    expect(screen.getByTestId("scheduled-task-select-t-b")).toBeTruthy();
  });

  // 19.17: fixes a real pre-19.16 bug -- Enable/Disable errors used to funnel into the (then
  // inline, now Sheet-only) create form's single shared `error` state, with no visible connection
  // to the row that actually failed.
  it("shows an Enable/Disable failure at the row that failed, not glued to an unrelated surface", async () => {
    mocks.listScheduledTasks.mockResolvedValueOnce([buildTask("t-a"), buildTask("t-b")]);
    mocks.setScheduledTaskEnabled.mockRejectedValueOnce(new Error("Agent is unavailable"));
    render(<ScheduledTasksPanel agents={agents} />);
    const rowA = (await screen.findByTestId("scheduled-task-select-t-a")).closest("li");
    if (!rowA) throw new Error("row t-a not found");

    fireEvent.click(within(rowA).getByRole("checkbox"));
    expect(await within(rowA).findByText("Agent is unavailable")).toBeTruthy();
    const rowB = screen.getByTestId("scheduled-task-select-t-b").closest("li");
    if (!rowB) throw new Error("row t-b not found");
    expect(within(rowB).queryByText("Agent is unavailable")).toBeNull();
  });

  // 19.10: Run now -- disabled while its own request is in flight, re-enabled once it settles,
  // and an error surfaces inline without disturbing the task's own displayed fields.
  describe("Run now", () => {
    async function selectTaskA() {
      mocks.listScheduledTasks.mockResolvedValueOnce([buildTask("t-a")]);
      render(<ScheduledTasksPanel agents={agents} />);
      fireEvent.click(await screen.findByTestId("scheduled-task-select-t-a"));
      const detail = screen.getByTestId("scheduled-task-detail");
      return within(detail).getByRole("button", { name: "Run now" }) as HTMLButtonElement;
    }

    it("calls the service with the selected task's id", async () => {
      mocks.runScheduledTaskNow.mockResolvedValueOnce({
        run: { id: "scheduled-run-1", taskId: "t-a", sessionId: "session-1", status: "succeeded", error: null, startedAt: "2026-08-31T09:00:00.000Z", completedAt: "2026-08-31T09:00:00.000Z" },
        operationId: null,
      });
      const button = await selectTaskA();

      fireEvent.click(button);

      await waitFor(() => expect(mocks.runScheduledTaskNow).toHaveBeenCalledWith("t-a"));
    });

    it("disables the button while the request is pending and re-enables it once it resolves", async () => {
      let resolveRun: (value: unknown) => void = () => {};
      mocks.runScheduledTaskNow.mockImplementation(() => new Promise((resolve) => { resolveRun = resolve; }));
      const button = await selectTaskA();

      fireEvent.click(button);
      await waitFor(() => expect(button.disabled).toBe(true));

      resolveRun({
        run: { id: "scheduled-run-1", taskId: "t-a", sessionId: "session-1", status: "succeeded", error: null, startedAt: "2026-08-31T09:00:00.000Z", completedAt: "2026-08-31T09:00:00.000Z" },
        operationId: null,
      });
      await waitFor(() => expect(button.disabled).toBe(false));
    });

    it("surfaces a rejected run inline without touching the task's own displayed status", async () => {
      mocks.runScheduledTaskNow.mockRejectedValueOnce(new Error("Agent is unavailable"));
      const button = await selectTaskA();
      const detail = screen.getByTestId("scheduled-task-detail");

      fireEvent.click(button);

      expect(await within(detail).findByText("Agent is unavailable")).toBeTruthy();
      expect(within(detail).getByText("Never run")).toBeTruthy();
      await waitFor(() => expect(button.disabled).toBe(false));
    });
  });
});
