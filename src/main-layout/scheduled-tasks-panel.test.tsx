// @vitest-environment jsdom

import { fireEvent, render, screen } from "@testing-library/react";
import { beforeAll, beforeEach, describe, expect, it, vi } from "vitest";
import { activateAppLanguage, i18n } from "../i18n";
import type { AgentRegistryEntry, ScheduledTask, ScheduledTaskFrequency } from "../types/agent";

const mocks = vi.hoisted(() => ({
  listScheduledTasks: vi.fn(),
}));

vi.mock("../services/runtime-agent-client", () => ({
  agentService: {
    listScheduledTasks: mocks.listScheduledTasks,
    createScheduledTask: vi.fn(),
    setScheduledTaskEnabled: vi.fn(),
    deleteScheduledTask: vi.fn(),
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
  };
}

const agents: AgentRegistryEntry[] = [
  { id: "onepiece", displayName: "OnePiece", supportedInteractionModes: ["cli"] } as AgentRegistryEntry,
];

describe("ScheduledTasksPanel", () => {
  beforeAll(async () => activateAppLanguage("en"));

  beforeEach(() => {
    mocks.listScheduledTasks.mockReset().mockResolvedValue([]);
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
});
