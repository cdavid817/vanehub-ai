// @vitest-environment jsdom

import { render, screen, within } from "@testing-library/react";
import { beforeAll, describe, expect, it } from "vitest";
import { activateAppLanguage, i18n } from "../i18n";
import type { ScheduledTaskFrequency } from "../types/agent";
import { ScheduledTaskOccurrences } from "./scheduled-task-occurrences";
import { formatDateTime } from "./scheduled-task-presentation";

const frequency: ScheduledTaskFrequency = { kind: "minutes", interval: 30 };
const nextRunAt = "2026-08-31T09:00:00.000Z";

describe("ScheduledTaskOccurrences", () => {
  beforeAll(async () => activateAppLanguage("en"));

  // 19.12: exactly five occurrences, the first of which is the task's own already-computed
  // `nextRunAt` verbatim -- not a freshly recomputed "next from now" that could disagree with it.
  it("previews exactly five occurrences, anchored at the task's own nextRunAt", () => {
    render(<ScheduledTaskOccurrences enabled frequency={frequency} language="en" nextRunAt={nextRunAt} />);
    const items = screen.getAllByRole("listitem");
    expect(items).toHaveLength(5);
    expect(items[0].textContent).toBe(formatDateTime(nextRunAt, "en"));
  });

  it("each occurrence is strictly later than the one before it", () => {
    render(<ScheduledTaskOccurrences enabled frequency={frequency} language="en" nextRunAt={nextRunAt} />);
    const items = within(screen.getByTestId("scheduled-task-occurrences")).getAllByRole("listitem");
    const times = items.map((item) => new Date(item.textContent ?? "").getTime());
    for (let index = 1; index < times.length; index += 1) {
      expect(times[index]).toBeGreaterThan(times[index - 1]);
    }
  });

  // A disabled task's own nextRunAt is stale (only `enabling` recomputes it) -- chaining a preview
  // from it would show times re-enabling would not actually reproduce, so this shows an honest
  // explanation instead of a fabricated chain.
  it("shows a disabled explanation instead of a preview when the task itself is disabled", () => {
    render(<ScheduledTaskOccurrences enabled={false} frequency={frequency} language="en" nextRunAt={nextRunAt} />);
    expect(screen.queryAllByRole("listitem")).toHaveLength(0);
    expect(screen.getByText(i18n.t("scheduledTasks.occurrences.disabled"))).toBeTruthy();
  });
});
