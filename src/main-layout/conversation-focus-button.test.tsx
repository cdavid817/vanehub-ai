// @vitest-environment jsdom

import { cleanup, fireEvent, render, screen } from "@testing-library/react";
import { afterEach, describe, expect, it, vi } from "vitest";
import "../i18n";
import { ConversationFocusButton } from "./conversation-focus-button";

afterEach(cleanup);

describe("ConversationFocusButton", () => {
  it("exposes localized enter and restore states", () => {
    const onToggle = vi.fn();
    const { rerender } = render(<ConversationFocusButton active={false} onToggle={onToggle} />);

    const enter = screen.getByRole("button", { name: "专注对话" });
    expect(enter.getAttribute("aria-pressed")).toBe("false");
    fireEvent.click(enter);
    expect(onToggle).toHaveBeenCalledOnce();

    rerender(<ConversationFocusButton active onToggle={onToggle} />);
    expect(screen.getByRole("button", { name: "恢复工作区" }).getAttribute("aria-pressed")).toBe("true");
  });
});
