// @vitest-environment jsdom

import { render, screen } from "@testing-library/react";
import { beforeAll, describe, expect, it, vi } from "vitest";
import { activateAppLanguage } from "../i18n";
import { generateScheduledTasks } from "../testing/fixtures/scheduled-task-fixtures";
import { ScheduledTaskList } from "./scheduled-task-list";

/**
 * 21.14 "100-task" budget. `generateScheduledTasks` (testing/fixtures/scheduled-task-fixtures.ts)
 * already exists for exactly this scale (task 0.9's own `FIXTURE_COUNTS.scheduledTasks = 100`,
 * confirmed via `large-scale-fixtures.ts`), but nothing drove it through the real list component
 * before this pass -- only `large-scale-fixtures.test.ts`'s own fixture-shape sanity check
 * (length/uniqueness/status coverage) consumed it, and no dedicated `scheduled-task-list.test.tsx`
 * existed at all.
 *
 * Disclosed, not built here: unlike Mission Control's Runs or Evaluation's arenas,
 * `agentService.listScheduledTasks()` takes no `cursor`/`limit` at all (confirmed by reading
 * `scheduled-task-service.ts`/`web-scheduled-task-client.ts`/`tauri-agent-client.ts`) -- there is
 * no service-side pagination to test here, and `ScheduledTaskList` renders every given task with no
 * cap of its own either. At the task-0.9-designed scale of 100 (an order of magnitude below
 * Sessions'/Work Items'/Goals' own 500-1,000-row budgets), this is a deliberate, accepted shape:
 * the budget worth proving is that the render stays exactly one row per task (no silent cap, no
 * duplication) and completes in a realistic time, not that a pagination/virtualization mechanism
 * exists.
 */
describe("ScheduledTaskList at scale (21.14)", () => {
  beforeAll(async () => activateAppLanguage("en"));

  it("renders exactly one row per task for a realistic 100-task fixture, with no cap or duplication", () => {
    const tasks = generateScheduledTasks(100);
    expect(new Set(tasks.map((task) => task.id)).size).toBe(100); // sanity: the fixture itself has no accidental id collision

    const start = performance.now();
    render(
      <ScheduledTaskList
        agents={[]}
        filtersActive={false}
        getMutation={() => undefined}
        language="en"
        loading={false}
        onDelete={vi.fn()}
        onDismissError={vi.fn()}
        onDuplicate={vi.fn()}
        onEdit={vi.fn()}
        onNew={vi.fn()}
        onSelect={vi.fn()}
        onSetEnabled={vi.fn()}
        selectedId={null}
        tasks={tasks}
        weekdayNames={["Sun", "Mon", "Tue", "Wed", "Thu", "Fri", "Sat"]}
      />,
    );
    const elapsedMs = performance.now() - start;
    console.info(`ScheduledTaskList 100-task render: ${elapsedMs.toFixed(1)}ms`);

    expect(screen.getAllByRole("listitem")).toHaveLength(100);
    const selectTestIds = new Set(tasks.map((task) => `scheduled-task-select-${task.id}`));
    for (const testId of selectTestIds) expect(screen.getByTestId(testId)).toBeTruthy();
  });
});
