// @vitest-environment jsdom

import { fireEvent, render, screen, waitFor, within } from "@testing-library/react";
import { beforeAll, beforeEach, describe, expect, it, vi } from "vitest";
import { activateAppLanguage, i18n } from "../i18n";
import type { AgentRegistryEntry, ScheduledTask, ScheduledTaskFrequency } from "../types/agent";

const mocks = vi.hoisted(() => ({
  listScheduledTasks: vi.fn(),
  runScheduledTaskNow: vi.fn(),
}));

vi.mock("../services/runtime-agent-client", () => ({
  agentService: {
    listScheduledTasks: mocks.listScheduledTasks,
    createScheduledTask: vi.fn(),
    setScheduledTaskEnabled: vi.fn(),
    deleteScheduledTask: vi.fn(),
    runScheduledTaskNow: mocks.runScheduledTaskNow,
  },
}));

import { ScheduledTasksPanel } from "./scheduled-tasks-panel";

function buildTask(id: string, frequency: ScheduledTaskFrequency): ScheduledTask {
  return {
    id,
    name: `Task ${id}`,
    content: "Do the thing",
    agentId: "onepiece",
    frequency,
    enabled: true,
    nextRunAt: "2026-08-31T09:00:00.000Z",
    latestStatus: "never-run",
    latestRunAt: null,
    latestRunSessionId: null,
    latestError: null,
    createdAt: "2026-08-01T00:00:00.000Z",
    updatedAt: "2026-08-01T00:00:00.000Z",
    version: 1,
  };
}

const agents: AgentRegistryEntry[] = [
  { id: "onepiece", displayName: "OnePiece", supportedInteractionModes: ["cli"] } as AgentRegistryEntry,
];

describe("ScheduledTasksPanel", () => {
  beforeAll(async () => activateAppLanguage("en"));

  beforeEach(() => {
    mocks.listScheduledTasks.mockReset().mockResolvedValue([]);
    mocks.runScheduledTaskNow.mockReset();
  });

  it("loads the task list on mount without needing a dialog `open` prop", async () => {
    render(<ScheduledTasksPanel agents={agents} />);
    expect(await screen.findByText("No scheduled tasks yet.")).toBeTruthy();
    expect(screen.getByRole("button", { name: /Create task/i })).toBeTruthy();
  });

  it("renders the frequency summary through i18next interpolation instead of a hard-coded string", async () => {
    await activateAppLanguage("en");
    mocks.listScheduledTasks.mockResolvedValueOnce([
      buildTask("t-weekly", { kind: "weekly", weekday: 1, timeOfDay: "09:00" }),
      buildTask("t-minute", { kind: "minutes", interval: 1 }),
    ]);
    render(<ScheduledTasksPanel agents={agents} />);
    // The weekday name comes from Intl (via formatAppWeekdayNames), not a hard-coded array, and
    // the lib only hands back the raw weekday index — this confirms the caller resolved it to
    // "Mon" and fed it through t() correctly.
    expect(await screen.findByText("Weekly Mon at 09:00")).toBeTruthy();
    // "Every 1 minute" (not the old "Every minute") is the intended i18next _one-plural outcome:
    // this codebase's plural keys always keep the number visible (see cli.installationsCount_one).
    expect(screen.getByText("Every 1 minute")).toBeTruthy();
  });

  it("shows locale-native weekday names in the weekday select instead of hard-coded English", async () => {
    await activateAppLanguage("ja");
    render(<ScheduledTasksPanel agents={agents} />);
    await screen.findByText(i18n.t("scheduledTasks.empty"));

    const frequencySelect = screen.getByLabelText(i18n.t("scheduledTasks.frequency")) as HTMLSelectElement;
    fireEvent.change(frequencySelect, { target: { value: "weekly" } });

    const weekdaySelect = screen.getByTestId("scheduled-task-weekday") as HTMLSelectElement;
    expect(Array.from(weekdaySelect.options).map((option) => option.textContent)).toEqual([
      "日", "月", "火", "水", "木", "金", "土",
    ]);

    await activateAppLanguage("en");
  });

  // 19.3: list/detail split -- selection is the one new piece of real behavior this task adds.
  it("selecting a task from the list shows it in the detail view", async () => {
    await activateAppLanguage("en");
    mocks.listScheduledTasks.mockResolvedValueOnce([
      buildTask("t-a", { kind: "daily", timeOfDay: "09:00" }),
      buildTask("t-b", { kind: "hours", interval: 2 }),
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

  // 19.3: `scheduleId` is `RunsSection`'s own field (workbench-route.ts) -- this is the first real
  // consumer of it (runs-destination.tsx wires it from the route; see that file's own test).
  it("uses the scheduleId prop as the initial selection, matching the route's own current task", async () => {
    await activateAppLanguage("en");
    mocks.listScheduledTasks.mockResolvedValueOnce([
      buildTask("t-a", { kind: "daily", timeOfDay: "09:00" }),
      buildTask("t-b", { kind: "hours", interval: 2 }),
    ]);
    render(<ScheduledTasksPanel agents={agents} scheduleId="t-b" />);
    expect(await within(await screen.findByTestId("scheduled-task-detail")).findByText("Task t-b")).toBeTruthy();
  });

  it("reports a selection change through onSelectSchedule so the route can be updated", async () => {
    await activateAppLanguage("en");
    mocks.listScheduledTasks.mockResolvedValueOnce([buildTask("t-a", { kind: "daily", timeOfDay: "09:00" })]);
    const onSelectSchedule = vi.fn();
    render(<ScheduledTasksPanel agents={agents} onSelectSchedule={onSelectSchedule} />);
    fireEvent.click(await screen.findByTestId("scheduled-task-select-t-a"));
    expect(onSelectSchedule).toHaveBeenCalledWith("t-a");
  });

  // 19.10: Run now -- disabled while its own request is in flight, re-enabled once it settles,
  // and an error surfaces inline without disturbing the task's own displayed fields.
  describe("Run now", () => {
    async function selectTaskA() {
      mocks.listScheduledTasks.mockResolvedValueOnce([buildTask("t-a", { kind: "daily", timeOfDay: "09:00" })]);
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
