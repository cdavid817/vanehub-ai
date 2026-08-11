// @vitest-environment jsdom

import { cleanup, fireEvent, render, screen } from "@testing-library/react";
import { afterEach, describe, expect, it, vi } from "vitest";
import "../i18n";
import { TopBar } from "./top-bar";

afterEach(cleanup);

describe("TopBar focus presentation", () => {
  it("contracts to a persistent restore surface in conversation focus mode", () => {
    const onFocusModeChange = vi.fn();
    render(<TopBar focusMode focusModeAvailable onFocusModeChange={onFocusModeChange} />);

    expect(screen.getByTestId("top-bar").getAttribute("data-focus-collapsed")).toBe("true");
    expect(screen.queryByPlaceholderText("搜索 Agent、对话、任务...")).toBeNull();
    fireEvent.click(screen.getByRole("button", { name: "恢复工作区" }));
    expect(onFocusModeChange).toHaveBeenCalledWith(false);
  });
});
