// @vitest-environment jsdom

import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { beforeEach, describe, expect, it, vi } from "vitest";
import "../i18n";
import { AgentTerminalComposer } from "./agent-terminal-composer";

describe("AgentTerminalComposer", () => {
  beforeEach(() => {
    window.localStorage.clear();
  });

  it("folds to one bar, keeps the draft, and remembers the choice", async () => {
    const user = userEvent.setup();
    const onSubmit = vi.fn();
    render(<AgentTerminalComposer canSubmit onSubmit={onSubmit} />);

    await user.type(screen.getByRole("textbox", { name: "工作区命令输入" }), "draft text");
    await user.click(screen.getByRole("button", { name: "收起输入框" }));
    expect(screen.queryByRole("textbox", { name: "工作区命令输入" })).toBeNull();
    expect(screen.getByText("可直接在终端中输入")).toBeTruthy();
    expect(window.localStorage.getItem("vanehub.agentTerminal.composerCollapsed")).toBe("1");

    await user.click(screen.getByRole("button", { name: "展开输入框" }));
    expect((screen.getByRole("textbox", { name: "工作区命令输入" }) as HTMLTextAreaElement).value).toBe("draft text");

    await user.click(screen.getByRole("textbox", { name: "工作区命令输入" }));
    await user.keyboard("{Enter}");
    expect(onSubmit).toHaveBeenCalledWith("draft text");
  });

  it("starts collapsed when the viewer collapsed it before", () => {
    window.localStorage.setItem("vanehub.agentTerminal.composerCollapsed", "1");
    render(<AgentTerminalComposer canSubmit onSubmit={vi.fn()} />);
    expect(screen.queryByRole("textbox", { name: "工作区命令输入" })).toBeNull();
    expect(screen.getByRole("button", { name: "展开输入框" }).getAttribute("aria-expanded")).toBe("false");
  });
});
