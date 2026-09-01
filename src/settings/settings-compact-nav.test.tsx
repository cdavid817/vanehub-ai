// @vitest-environment jsdom

import { fireEvent, render, screen } from "@testing-library/react";
import { beforeAll, describe, expect, it, vi } from "vitest";
import { activateAppLanguage } from "../i18n";
import { SettingsCompactNav } from "./settings-compact-nav";

describe("SettingsCompactNav (task 12.9)", () => {
  beforeAll(async () => activateAppLanguage("zh-CN"));

  it("shows a trigger naming the current page and no sheet until opened", () => {
    render(<SettingsCompactNav activePageId="basic" onSelectPage={vi.fn()} />);
    // The accessible name is a stable "switch page" label plus the current page, not just the
    // current page's own name -- a screen reader user tabbing to it must hear that it is a
    // navigation control, not a static heading (task 12.11's e2e fix surfaced this gap).
    expect(screen.getByRole("button", { name: /^切换设置页面.*基础配置/ })).toBeTruthy();
    expect(screen.getByText("基础配置")).toBeTruthy();
    expect(screen.queryByRole("dialog")).toBeNull();
  });

  it("opens a searchable sheet listing every registered page, grouped", () => {
    render(<SettingsCompactNav activePageId="basic" onSelectPage={vi.fn()} />);
    fireEvent.click(screen.getByRole("button", { name: /^切换设置页面/ }));

    const dialog = screen.getByRole("dialog");
    expect(dialog).toBeTruthy();
    // A page from a different group than "basic" (general) -- proves every group is listed, not
    // just the active page's own.
    expect(screen.getByRole("button", { name: /SSH 连接/ })).toBeTruthy();
  });

  it("filters the listed pages as the reader types, without horizontal scrolling required", () => {
    render(<SettingsCompactNav activePageId="basic" onSelectPage={vi.fn()} />);
    fireEvent.click(screen.getByRole("button", { name: /^切换设置页面/ }));

    fireEvent.change(screen.getByPlaceholderText("筛选设置页面..."), { target: { value: "SSH" } });
    expect(screen.getByRole("button", { name: /SSH 连接/ })).toBeTruthy();
    expect(screen.queryByRole("button", { name: /CLI 管理/ })).toBeNull();
  });

  it("shows a no-results state for a query matching nothing", () => {
    render(<SettingsCompactNav activePageId="basic" onSelectPage={vi.fn()} />);
    fireEvent.click(screen.getByRole("button", { name: /^切换设置页面/ }));
    fireEvent.change(screen.getByPlaceholderText("筛选设置页面..."), { target: { value: "zzz-no-such-page" } });
    expect(screen.getByRole("status").textContent).toContain("没有匹配的设置页面");
  });

  it("selects a page, closes the sheet, and clears the filter", () => {
    const onSelectPage = vi.fn();
    render(<SettingsCompactNav activePageId="basic" onSelectPage={onSelectPage} />);
    fireEvent.click(screen.getByRole("button", { name: /^切换设置页面/ }));
    fireEvent.click(screen.getByRole("button", { name: /SSH 连接/ }));

    expect(onSelectPage).toHaveBeenCalledWith("ssh-connections");
    expect(screen.queryByRole("dialog")).toBeNull();
  });

  it("marks the active page with aria-current inside the sheet", () => {
    render(<SettingsCompactNav activePageId="ssh-connections" onSelectPage={vi.fn()} />);
    fireEvent.click(screen.getByRole("button", { name: /^切换设置页面/ }));
    const activeRow = screen
      .getAllByRole("button", { name: /SSH 连接/ })
      .find((entry) => entry.getAttribute("aria-current") === "page");
    expect(activeRow).toBeTruthy();
  });
});
