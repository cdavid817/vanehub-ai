// @vitest-environment jsdom

import { fireEvent, render, screen } from "@testing-library/react";
import { beforeAll, describe, expect, it, vi } from "vitest";
import { activateAppLanguage, i18n } from "../i18n";
import { ScheduledTaskSessionLink } from "./scheduled-task-session-link";

describe("ScheduledTaskSessionLink", () => {
  beforeAll(async () => activateAppLanguage("en"));

  it("renders a dash when there is no session id", () => {
    render(<ScheduledTaskSessionLink sessionId={null} />);
    expect(screen.getByText("—")).toBeTruthy();
  });

  // 19.11/19.6: without a caller-supplied onOpenSession (this task batch's own current reality --
  // see this component's own doc comment for why), the session id is still shown as plain text
  // rather than silently dropped, just not falsely clickable.
  it("shows the session id as plain, non-interactive text when no onOpenSession callback is supplied", () => {
    render(<ScheduledTaskSessionLink sessionId="session-42" />);
    expect(screen.getByText("session-42")).toBeTruthy();
    expect(screen.queryByRole("button")).toBeNull();
  });

  it("renders an actionable button that calls onOpenSession with the session id when a callback is supplied", () => {
    const onOpenSession = vi.fn();
    render(<ScheduledTaskSessionLink onOpenSession={onOpenSession} sessionId="session-42" />);
    fireEvent.click(screen.getByRole("button", { name: i18n.t("scheduledTasks.history.openSession") }));
    expect(onOpenSession).toHaveBeenCalledWith("session-42");
  });
});
