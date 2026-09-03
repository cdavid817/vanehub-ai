// @vitest-environment jsdom

import { fireEvent, render, screen, waitFor, within } from "@testing-library/react";
import { beforeAll, describe, expect, it, vi, type Mock } from "vitest";
import { activateAppLanguage, i18n } from "../i18n";
import type { AgentRegistryEntry, ScheduledTask } from "../types/agent";
import type { MutationState } from "../ui/async/mutation-state";
import { ScheduledTaskEditorSheet, type ScheduledTaskEditorMode } from "./scheduled-task-editor-sheet";

const agents: AgentRegistryEntry[] = [
  { id: "onepiece", displayName: "OnePiece", supportedInteractionModes: ["cli"] } as AgentRegistryEntry,
];
const weekdayNames = ["Sun", "Mon", "Tue", "Wed", "Thu", "Fri", "Sat"];

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
    version: 3,
    ...overrides,
  };
}

interface SheetMocks {
  onClose: Mock;
  onCreate: Mock;
  onCreated: Mock;
  onReload: Mock;
  onUpdate: Mock;
}

// Explicit `Mock`-typed locals, not a generic `Partial<Parameters<...>>` override merge: spreading
// an override object typed against the component's own (non-Mock) prop types collapses each
// function prop's inferred type to a union that loses `vi.fn()`'s own `mockResolvedValueOnce`
// etc., which every test below needs.
function renderSheet(mode: ScheduledTaskEditorMode, mutation?: MutationState): SheetMocks {
  const mocks: SheetMocks = { onClose: vi.fn(), onCreate: vi.fn(), onCreated: vi.fn(), onReload: vi.fn(), onUpdate: vi.fn() };
  render(
    <ScheduledTaskEditorSheet
      agents={agents}
      mode={mode}
      mutation={mutation}
      onClose={mocks.onClose}
      onCreate={mocks.onCreate}
      onCreated={mocks.onCreated}
      onReload={mocks.onReload}
      onUpdate={mocks.onUpdate}
      weekdayNames={weekdayNames}
    />,
  );
  return mocks;
}

describe("ScheduledTaskEditorSheet", () => {
  beforeAll(async () => activateAppLanguage("en"));

  describe("create mode", () => {
    it("starts blank, with Save disabled until name/content/agent are filled", async () => {
      const props = renderSheet({ kind: "create" });
      expect(screen.getByRole("heading", { name: "New task" })).toBeTruthy();
      const save = screen.getByRole("button", { name: "Create task" });
      expect(save.getAttribute("disabled")).not.toBeNull();
      expect(screen.getByText("Task name is required.")).toBeTruthy();

      fireEvent.change(screen.getByLabelText(i18n.t("scheduledTasks.name")), { target: { value: "New task" } });
      fireEvent.change(screen.getByLabelText(i18n.t("scheduledTasks.content")), { target: { value: "Do it" } });
      expect(save.getAttribute("disabled")).toBeNull();

      props.onCreate.mockResolvedValueOnce(buildTask({ id: "t-new", name: "New task" }));
      fireEvent.click(save);
      await waitFor(() => expect(props.onCreate).toHaveBeenCalledWith({ agentId: "onepiece", content: "Do it", frequency: { kind: "daily", timeOfDay: "09:00" }, name: "New task" }));
      await waitFor(() => expect(props.onCreated).toHaveBeenCalledWith(expect.objectContaining({ id: "t-new" })));
      expect(props.onClose).toHaveBeenCalledOnce();
    });

    it("does not call onCreated/onClose when the create request is rejected", async () => {
      const props = renderSheet({ kind: "create" });
      fireEvent.change(screen.getByLabelText(i18n.t("scheduledTasks.name")), { target: { value: "New task" } });
      fireEvent.change(screen.getByLabelText(i18n.t("scheduledTasks.content")), { target: { value: "Do it" } });
      props.onCreate.mockRejectedValueOnce(new Error("Scheduled task name is required."));

      fireEvent.click(screen.getByRole("button", { name: "Create task" }));
      await waitFor(() => expect(props.onCreate).toHaveBeenCalledOnce());
      expect(props.onCreated).not.toHaveBeenCalled();
      expect(props.onClose).not.toHaveBeenCalled();
    });

    it("Cancel closes without creating anything", () => {
      const props = renderSheet({ kind: "create" });
      fireEvent.click(screen.getByRole("button", { name: "Cancel" }));
      expect(props.onClose).toHaveBeenCalledOnce();
      expect(props.onCreate).not.toHaveBeenCalled();
    });
  });

  describe("edit mode", () => {
    it("prefills every field from the task and titles the sheet with its name", () => {
      const task = buildTask();
      renderSheet({ kind: "edit", task });
      expect(screen.getByRole("heading", { name: "Edit task Nightly digest" })).toBeTruthy();
      expect((screen.getByLabelText(i18n.t("scheduledTasks.name")) as HTMLInputElement).value).toBe("Nightly digest");
      expect((screen.getByLabelText(i18n.t("scheduledTasks.content")) as HTMLTextAreaElement).value).toBe("Summarize commits");
    });

    it("Save calls onUpdate with the task's own version as expectedVersion, then closes", async () => {
      const task = buildTask({ version: 5 });
      const props = renderSheet({ kind: "edit", task });
      fireEvent.change(screen.getByLabelText(i18n.t("scheduledTasks.name")), { target: { value: "Renamed digest" } });
      props.onUpdate.mockResolvedValueOnce({ ...task, name: "Renamed digest", version: 6 });

      fireEvent.click(screen.getByRole("button", { name: "Save" }));
      await waitFor(() => expect(props.onUpdate).toHaveBeenCalledWith(task, {
        agentId: "onepiece", content: "Summarize commits", expectedVersion: 5, frequency: { kind: "daily", timeOfDay: "09:00" }, name: "Renamed digest", taskId: "t-a",
      }));
      expect(props.onClose).toHaveBeenCalledOnce();
    });

    // 19.7: on conflict, refetch, explain, and let the reader retry against the fresh version --
    // never silently overwrite what they typed.
    it("on a version conflict that still exists, explains it, reloads, and retries with the fresh version -- without discarding the draft", async () => {
      const task = buildTask({ version: 5 });
      const refreshed = buildTask({ version: 6, name: "Renamed elsewhere" });
      const props = renderSheet({ kind: "edit", task });
      fireEvent.change(screen.getByLabelText(i18n.t("scheduledTasks.name")), { target: { value: "My local edit" } });
      props.onUpdate.mockRejectedValueOnce(new Error("scheduled-task-version-conflict: expected 5, stored 6"));
      props.onReload.mockResolvedValueOnce([refreshed]);

      fireEvent.click(screen.getByRole("button", { name: "Save" }));
      expect(await screen.findByText(/changed elsewhere/)).toBeTruthy();
      // The reader's own typed edit survives -- it is never replaced by the server's "Renamed
      // elsewhere" value.
      expect((screen.getByLabelText(i18n.t("scheduledTasks.name")) as HTMLInputElement).value).toBe("My local edit");

      props.onUpdate.mockResolvedValueOnce({ ...refreshed, name: "My local edit", version: 7 });
      fireEvent.click(screen.getByRole("button", { name: "Save" }));
      await waitFor(() => expect(props.onUpdate).toHaveBeenLastCalledWith(refreshed, expect.objectContaining({ expectedVersion: 6, name: "My local edit" })));
    });

    it("on a version conflict where the task was deleted elsewhere, explains it and disables Save", async () => {
      const task = buildTask({ version: 5 });
      const props = renderSheet({ kind: "edit", task });
      props.onUpdate.mockRejectedValueOnce(new Error("scheduled-task-version-conflict: expected 5, stored 6"));
      props.onReload.mockResolvedValueOnce([]);

      fireEvent.click(screen.getByRole("button", { name: "Save" }));
      expect(await screen.findByText(/deleted elsewhere/)).toBeTruthy();
      expect(screen.getByRole("button", { name: "Save" }).getAttribute("disabled")).not.toBeNull();
    });
  });

  describe("duplicate mode", () => {
    it("prefills from the source task with an adjusted name, and submits through onCreate (not onUpdate)", async () => {
      const source = buildTask({ id: "t-source", name: "Nightly digest" });
      const props = renderSheet({ kind: "duplicate", source });
      expect(screen.getByRole("heading", { name: "New task" })).toBeTruthy();
      expect((screen.getByLabelText(i18n.t("scheduledTasks.name")) as HTMLInputElement).value).toBe("Nightly digest copy");
      expect((screen.getByLabelText(i18n.t("scheduledTasks.content")) as HTMLTextAreaElement).value).toBe("Summarize commits");

      props.onCreate.mockResolvedValueOnce(buildTask({ id: "t-dup", name: "Nightly digest copy" }));
      fireEvent.click(screen.getByRole("button", { name: "Create task" }));
      await waitFor(() => expect(props.onCreate).toHaveBeenCalledWith(expect.objectContaining({ name: "Nightly digest copy" })));
      expect(props.onUpdate).not.toHaveBeenCalled();
    });
  });

  it("shows the Review section restating name/agent/frequency/content live as the draft changes", () => {
    renderSheet({ kind: "create" });
    fireEvent.change(screen.getByLabelText(i18n.t("scheduledTasks.name")), { target: { value: "Weekly report" } });
    const review = screen.getByRole("region", { name: "Review" });
    expect(review.textContent).toContain("Weekly report");
  });

  // 19.13/19.15: Review restates the same honest execution facts the create/edit form already
  // shows during editing (`ScheduledTaskForm`'s own `runtimeHint` paragraph) -- shown once more
  // here so the reader sees them again at the exact point they actually commit.
  it("shows the honest execution notice (device timezone + catch-up model) inside Review", () => {
    renderSheet({ kind: "create" });
    const review = screen.getByRole("region", { name: "Review" });
    expect(within(review).getByTestId("scheduled-task-execution-notice")).toBeTruthy();
  });
});
