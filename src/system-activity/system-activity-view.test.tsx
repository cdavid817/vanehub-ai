// @vitest-environment jsdom

import { screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import type { ReactNode } from "react";
import { beforeEach, describe, expect, it, vi } from "vitest";
import { activateAppLanguage } from "../i18n";
import { renderWithAppProviders } from "../test/render";
import {
  resetWebSystemActivityForTest,
  seedWebSystemActivityEventForTest,
} from "../services/web-system-activity-state";
import { SystemActivityView } from "./system-activity-view";

vi.mock("../components/measured-virtual-list", () => ({
  MeasuredVirtualList: <T,>({ items, renderItem, testId }: {
    items: readonly T[];
    renderItem: (item: T, index: number) => ReactNode;
    testId?: string;
  }) => (
    <div data-testid={testId} data-virtual-count={items.length} role="list">
      {items.map(renderItem)}
    </div>
  ),
}));

describe("SystemActivityView", () => {
  beforeEach(async () => {
    resetWebSystemActivityForTest();
    await activateAppLanguage("zh-CN");
  });

  it("shows the empty state before any committed activity exists", async () => {
    renderWithAppProviders(<SystemActivityView />);
    expect(await screen.findByTestId("system-activity-empty")).toBeTruthy();
  });

  it("lists lazy sessions with unread badges and renders read-only timeline items", async () => {
    seedWebSystemActivityEventForTest("workspace", "workspace-one", "run_completed");
    seedWebSystemActivityEventForTest("workspace", "workspace-one", "breaker_opened", "error");
    renderWithAppProviders(<SystemActivityView />);

    expect(await screen.findByTestId("system-activity-session")).toBeTruthy();
    expect(screen.getByTestId("system-activity-unread-badge").textContent).toBe("2");
    await waitFor(() => {
      expect(screen.getAllByTestId("system-activity-item")).toHaveLength(2);
    });
    expect(screen.getByTestId("system-activity-timeline").dataset.virtualCount).toBe("2");
    // Localized at render time from the locale-neutral event code.
    expect(screen.getByText("运行已完成")).toBeTruthy();
    expect(screen.getByText("熔断器已打开")).toBeTruthy();
    // The view is not an interactive chat: no composer, textbox for messages, or send control.
    expect(screen.queryByRole("textbox", { name: /composer|message/i })).toBeNull();
    expect(document.querySelector("[data-testid='chat-composer']")).toBeNull();
  });

  it("marks the timeline read without deleting any activity", async () => {
    seedWebSystemActivityEventForTest("workspace", "workspace-one", "run_completed");
    renderWithAppProviders(<SystemActivityView />);
    await screen.findByTestId("system-activity-item");

    await userEvent.click(screen.getByTestId("system-activity-mark-read"));
    await waitFor(() => {
      expect(screen.queryByTestId("system-activity-unread-badge")).toBeNull();
    });
    expect(screen.getAllByTestId("system-activity-item")).toHaveLength(1);
  });

  it("filters by severity without mutating the underlying timeline", async () => {
    seedWebSystemActivityEventForTest("workspace", "workspace-one", "run_completed");
    seedWebSystemActivityEventForTest("workspace", "workspace-one", "breaker_opened", "error");
    renderWithAppProviders(<SystemActivityView />);
    await screen.findAllByTestId("system-activity-item");

    await userEvent.selectOptions(
      screen.getByRole("combobox", { name: "按严重级别筛选" }),
      "error",
    );
    await waitFor(() => {
      expect(screen.getAllByTestId("system-activity-item")).toHaveLength(1);
    });
    await userEvent.selectOptions(screen.getByRole("combobox", { name: "按严重级别筛选" }), "");
    await waitFor(() => {
      expect(screen.getAllByTestId("system-activity-item")).toHaveLength(2);
    });
  });

  it("exposes localized landmarks and accessible names for every control", async () => {
    seedWebSystemActivityEventForTest("workspace", "workspace-one", "run_completed");
    renderWithAppProviders(<SystemActivityView />);
    await screen.findAllByTestId("system-activity-item");

    expect(screen.getByRole("navigation", { name: "系统活动会话" })).toBeTruthy();
    expect(screen.getByRole("region", { name: "活动时间线" })).toBeTruthy();
    expect(screen.getByRole("textbox", { name: "搜索事件或安全标识" })).toBeTruthy();
    expect(screen.getByRole("combobox", { name: "按严重级别筛选" })).toBeTruthy();
    // Every interactive control is a real button reachable by keyboard, never a click-div.
    for (const button of screen.getAllByRole("button")) {
      expect(button.tagName).toBe("BUTTON");
    }
  });

  it("re-renders persisted history in a newly selected locale", async () => {
    seedWebSystemActivityEventForTest("workspace", "workspace-one", "run_completed");
    renderWithAppProviders(<SystemActivityView />);
    expect(await screen.findByText("运行已完成")).toBeTruthy();

    await activateAppLanguage("en");
    await waitFor(() => {
      expect(screen.getByText("Run completed")).toBeTruthy();
    });
  });
});
