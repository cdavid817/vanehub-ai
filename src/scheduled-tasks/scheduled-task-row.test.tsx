// @vitest-environment jsdom

import { fireEvent, render, screen, waitFor } from "@testing-library/react";
import { beforeAll, describe, expect, it, vi } from "vitest";
import { activateAppLanguage } from "../i18n";
import type { AgentRegistryEntry, ScheduledTask } from "../types/agent";
import { ScheduledTaskRow } from "./scheduled-task-row";

const agent: AgentRegistryEntry = { id: "onepiece", displayName: "OnePiece", supportedInteractionModes: ["cli"] } as AgentRegistryEntry;

function buildTask(overrides: Partial<ScheduledTask> = {}): ScheduledTask {
  return {
    id: "t-a",
    name: "Nightly digest",
    content: "Summarize commits",
    agentId: "onepiece",
    frequency: { kind: "daily", timeOfDay: "09:00" },
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

function renderRow(overrides: Partial<Parameters<typeof ScheduledTaskRow>[0]> = {}) {
  const props = {
    agent,
    language: "en",
    onDelete: vi.fn(),
    onDismissError: vi.fn(),
    onDuplicate: vi.fn(),
    onEdit: vi.fn(),
    onSelect: vi.fn(),
    onSetEnabled: vi.fn(),
    selected: false,
    task: buildTask(),
    weekdayNames: ["Sun", "Mon", "Tue", "Wed", "Thu", "Fri", "Sat"],
    ...overrides,
  };
  render(<ul><ScheduledTaskRow {...props} /></ul>);
  return props;
}

describe("ScheduledTaskRow", () => {
  beforeAll(async () => activateAppLanguage("en"));

  it("selecting the row's name/agent button calls onSelect with the task id", () => {
    const props = renderRow();
    fireEvent.click(screen.getByTestId("scheduled-task-select-t-a"));
    expect(props.onSelect).toHaveBeenCalledWith("t-a");
  });

  it("toggling the checkbox calls onSetEnabled with the new value", () => {
    const props = renderRow();
    fireEvent.click(screen.getByRole("checkbox"));
    expect(props.onSetEnabled).toHaveBeenCalledWith(props.task, false);
  });

  // 19.16: Edit and Duplicate run directly, with no confirmation -- only Delete is
  // consequence-aware confirmed.
  it("More > Edit calls onEdit directly, without confirmation", () => {
    const props = renderRow();
    fireEvent.click(screen.getByRole("button", { name: "More actions" }));
    fireEvent.click(screen.getByRole("menuitem", { name: "Edit task" }));
    expect(props.onEdit).toHaveBeenCalledWith(props.task);
    expect(screen.queryByRole("dialog")).toBeNull();
  });

  it("More > Duplicate calls onDuplicate directly, without confirmation", () => {
    const props = renderRow();
    fireEvent.click(screen.getByRole("button", { name: "More actions" }));
    fireEvent.click(screen.getByRole("menuitem", { name: "Duplicate task" }));
    expect(props.onDuplicate).toHaveBeenCalledWith(props.task);
    expect(screen.queryByRole("dialog")).toBeNull();
  });

  it("More > Delete requires confirmation naming the task, with an honest consequence description, before calling onDelete", async () => {
    const props = renderRow();
    fireEvent.click(screen.getByRole("button", { name: "More actions" }));
    fireEvent.click(screen.getByRole("menuitem", { name: "Delete task" }));

    expect(await screen.findByRole("dialog")).toBeTruthy();
    expect(screen.getByText('Delete scheduled task "Nightly digest"?')).toBeTruthy();
    // Honest claim: the task stops running and cannot be recovered -- it does not claim run
    // history is preserved or viewable (19.11 is what would make that claim true, and it is not
    // built yet), nor does it claim history is deleted (delete_scheduled_task never touches
    // scheduled_task_runs).
    expect(screen.getByText("This permanently removes the task. It stops running on its schedule and cannot be recovered.")).toBeTruthy();
    expect(props.onDelete).not.toHaveBeenCalled();

    fireEvent.click(screen.getByRole("button", { name: "Confirm delete" }));
    await waitFor(() => expect(props.onDelete).toHaveBeenCalledWith(props.task));
  });

  it("cancelling the delete confirmation never calls onDelete", async () => {
    const props = renderRow();
    fireEvent.click(screen.getByRole("button", { name: "More actions" }));
    fireEvent.click(screen.getByRole("menuitem", { name: "Delete task" }));
    fireEvent.click(await screen.findByRole("button", { name: "Cancel" }));
    expect(props.onDelete).not.toHaveBeenCalled();
  });

  // 19.17: this row's own mutation disables all of this row's own controls together (Enable/
  // Disable and every More item) -- see use-scheduled-tasks-actions.ts's own doc comment for why.
  it("disables Enable/Disable and every More item while this row's own mutation is pending", () => {
    renderRow({ mutation: { targetKey: "t-a", pending: true } });
    expect((screen.getByRole("checkbox") as HTMLInputElement).disabled).toBe(true);
    fireEvent.click(screen.getByRole("button", { name: "More actions" }));
    for (const name of ["Edit task", "Duplicate task", "Delete task"]) {
      expect(screen.getByRole("menuitem", { name }).getAttribute("aria-disabled")).toBe("true");
    }
  });

  it("shows this row's own mutation error and lets it be dismissed", () => {
    const props = renderRow({ mutation: { targetKey: "t-a", pending: false, error: { kind: "error", message: "Agent unavailable", retryable: false } } });
    expect(screen.getByText("Agent unavailable")).toBeTruthy();
    fireEvent.click(screen.getByRole("button", { name: "Dismiss" }));
    expect(props.onDismissError).toHaveBeenCalledWith("t-a");
  });
});
