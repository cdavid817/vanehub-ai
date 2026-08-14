// @vitest-environment jsdom

import { cleanup, fireEvent, render, screen } from "@testing-library/react";
import { afterEach, describe, expect, it, vi } from "vitest";
import "../i18n";
import { NotificationProvider } from "../notifications/notification-provider";
import { TopBar } from "./top-bar";

afterEach(cleanup);

describe("TopBar focus presentation", () => {
  it("contracts to a persistent restore surface in conversation focus mode", () => {
    const onFocusModeChange = vi.fn();
    render(<TopBar focusMode focusModeAvailable onFocusModeChange={onFocusModeChange} onSearch={vi.fn()} />);

    expect(screen.getByTestId("top-bar").getAttribute("data-focus-collapsed")).toBe("true");
    expect(screen.queryByRole("button", { name: "打开搜索" })).toBeNull();
    fireEvent.click(screen.getByRole("button", { name: "恢复工作区" }));
    expect(onFocusModeChange).toHaveBeenCalledWith(false);
  });

  /**
   * The entry used to open a second input with no value binding or submit path. It has to reach
   * the session search that actually runs a query instead of rendering its own dead field.
   */
  it("delegates search to the session sidebar instead of opening its own input", () => {
    const onSearch = vi.fn();
    render(
      // The expanded top bar mounts the notification centre, which needs its provider.
      <NotificationProvider>
        <TopBar focusMode={false} focusModeAvailable onFocusModeChange={vi.fn()} onSearch={onSearch} />
      </NotificationProvider>,
    );

    const trigger = screen.getByRole("button", { name: "打开搜索" });
    expect(trigger.getAttribute("aria-controls")).toBe("workspace-session-search");
    fireEvent.click(trigger);

    expect(onSearch).toHaveBeenCalledOnce();
    expect(screen.queryByRole("textbox")).toBeNull();
  });
});
