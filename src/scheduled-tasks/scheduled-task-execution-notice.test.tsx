// @vitest-environment jsdom

import { render, screen } from "@testing-library/react";
import { beforeAll, describe, expect, it } from "vitest";
import { activateAppLanguage, i18n } from "../i18n";
import { ScheduledTaskExecutionNotice } from "./scheduled-task-execution-notice";

describe("ScheduledTaskExecutionNotice", () => {
  beforeAll(async () => activateAppLanguage("en"));

  // 19.13: the real, honest fact (this device's own current IANA zone) instead of a fabricated
  // configured-timezone selector -- computed the same way the component itself does, not
  // hard-coded to a specific zone name, so this stays portable across machines/CI runners.
  it("shows this device's own real timezone, not a configured/stored one", () => {
    render(<ScheduledTaskExecutionNotice />);
    const zone = Intl.DateTimeFormat().resolvedOptions().timeZone;
    expect(screen.getByText(i18n.t("scheduledTasks.executionNotice.timezone", { zone }))).toBeTruthy();
  });

  // 19.15: reuses the exact same key `ScheduledTaskForm` already shows during editing, so the
  // form, Review, and detail view can never quietly disagree about the same real behavior.
  it("reuses the existing runtimeHint key for the catch-up execution model instead of a second paraphrase", () => {
    render(<ScheduledTaskExecutionNotice />);
    expect(screen.getByText(i18n.t("scheduledTasks.runtimeHint"))).toBeTruthy();
  });
});
