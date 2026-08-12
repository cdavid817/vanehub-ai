// @vitest-environment jsdom

import { fireEvent, render, screen } from "@testing-library/react";
import { afterEach, describe, expect, it, vi } from "vitest";
import "../../i18n";
import { activateAppLanguage } from "../../i18n";
import { ToolUseBlock, toolActivityPreview } from "./ToolUseBlock";

vi.mock("../../services/runtime-permissions-client", () => ({
  permissionsService: {
    listPendingApprovals: vi.fn().mockResolvedValue([]),
    resolvePendingApproval: vi.fn().mockResolvedValue(undefined),
  },
}));

afterEach(async () => {
  await activateAppLanguage("zh-CN");
});

describe("ToolUseBlock", () => {
  it("normalizes duplicate ids and orders actionable work before completed history", () => {
    const { container } = render(
      <ToolUseBlock
        sessionId="session-1"
        toolUse={[
          { id: "done", name: "read_file", input: { path: "README.md" }, status: "completed" },
          { id: "shell", name: "shell", input: { command: "npm test" }, status: "running" },
          { id: "failed", name: "glob", input: { pattern: "src/**/*.tsx" }, status: "failed" },
          { id: "shell", name: "shell", output: "passed", status: "completed" },
          { id: "approval", name: "file", input: { path: "package.json" }, status: "awaiting_approval" },
        ]}
      />,
    );

    expect(container.querySelectorAll("[data-tool-status]")).toHaveLength(4);
    expect([...container.querySelectorAll("[data-tool-status]")].map((node) => node.getAttribute("data-tool-status"))).toEqual([
      "awaiting_approval", "failed", "completed", "completed",
    ]);
    expect(screen.getByTestId("completed-tool-history").hasAttribute("open")).toBe(false);
    expect(screen.getByText("已完成 2 项")).toBeTruthy();
    expect(screen.getByText("该工具调用需要你的确认才能执行")).toBeTruthy();
  });

  it("renders localized status summaries and friendly tool names", async () => {
    await activateAppLanguage("en");
    render(<ToolUseBlock sessionId="session-1" toolUse={[{ id: "shell", name: "shell", input: { command: "git status" }, status: "running" }]} />);
    expect(screen.getByRole("region", { name: "Tool activity" })).toBeTruthy();
    expect(screen.getByText("1 active")).toBeTruthy();
    expect(screen.getByText("Shell command")).toBeTruthy();
    expect(screen.getByText("git status")).toBeTruthy();
  });

  it("identifies one completed activity in the collapsed summary", () => {
    render(<ToolUseBlock messageStatus="completed" sessionId="session-1" toolUse={[{ id: "read", name: "read_file", input: { path: "README.md" }, status: "completed" }]} />);
    expect(screen.getAllByText("读取文件")).toHaveLength(3);
    expect(screen.getAllByText("README.md")).toHaveLength(3);
    expect(screen.getByRole("button", { name: "展开工具活动" }).getAttribute("aria-expanded")).toBe("false");
    expect(screen.getByTestId("tool-activity-content").hasAttribute("hidden")).toBe(true);
    expect(screen.getByTestId("completed-tool-history").hasAttribute("open")).toBe(false);
  });

  it("lets active work collapse manually and retains that choice after completion", () => {
    const running = [{ id: "shell", name: "shell", input: { command: "npm test" }, status: "running" as const }];
    const { rerender } = render(<ToolUseBlock messageStatus="streaming" sessionId="session-1" toolUse={running} />);
    const toggle = screen.getByRole("button", { name: "收起工具活动" });
    expect(toggle.getAttribute("aria-expanded")).toBe("true");

    fireEvent.click(toggle);
    expect(screen.getByRole("button", { name: "展开工具活动" }).getAttribute("aria-expanded")).toBe("false");
    expect(screen.getAllByText("npm test").length).toBeGreaterThan(0);

    rerender(<ToolUseBlock messageStatus="completed" sessionId="session-1" toolUse={[{ ...running[0], output: "passed", status: "completed" }]} />);
    expect(screen.getByRole("button", { name: "展开工具活动" }).getAttribute("aria-expanded")).toBe("false");
  });

  it("forces approval controls open over a collapsed preference", () => {
    const { rerender } = render(
      <ToolUseBlock messageStatus="streaming" sessionId="session-1" toolUse={[{ id: "shell", name: "shell", input: { command: "npm test" }, status: "running" }]} />,
    );
    fireEvent.click(screen.getByRole("button", { name: "收起工具活动" }));

    rerender(
      <ToolUseBlock messageStatus="streaming" sessionId="session-1" toolUse={[{ id: "shell", name: "shell", input: { command: "npm test" }, status: "awaiting_approval" }]} />,
    );
    const toggle = screen.getByRole("button", { name: "收起工具活动" });
    expect(toggle.getAttribute("aria-expanded")).toBe("true");
    expect(toggle.hasAttribute("disabled")).toBe(true);
    expect(screen.getByText("该工具调用需要你的确认才能执行")).toBeTruthy();
  });

  it("collapses recoverable failures by default", () => {
    render(
      <ToolUseBlock
        sessionId="session-1"
        toolUse={[{ id: "failed-1", name: "shell", input: { command: "npm test" }, output: "exit code 1", status: "failed" }]}
      />,
    );

    expect(screen.getByText("失败 1")).toBeTruthy();
    expect(screen.getByTestId("failed-tool-history").hasAttribute("open")).toBe(false);
    expect(screen.getByText("1 个失败操作")).toBeTruthy();
  });

  it("initially discloses failures when the assistant message failed", () => {
    render(
      <ToolUseBlock
        messageFailed
        messageStatus="failed"
        sessionId="session-1"
        toolUse={[{ id: "failed-1", name: "shell", input: { command: "cargo check" }, output: "exit code 1", status: "failed" }]}
      />,
    );

    expect(screen.getByTestId("failed-tool-history").hasAttribute("open")).toBe(true);
    expect(screen.getAllByText("cargo check").length).toBeGreaterThan(0);
  });

  it("groups consecutive identical failures without dropping occurrence payloads", () => {
    const { container } = render(
      <ToolUseBlock
        sessionId="session-1"
        toolUse={[
          { id: "failed-1", name: "shell", input: { command: "npm test" }, output: "exit code 1", status: "failed" },
          { id: "failed-2", name: "shell", input: { command: "npm test" }, output: "exit code 1", status: "failed" },
        ]}
      />,
    );

    expect(container.querySelectorAll('[data-tool-status="failed"]')).toHaveLength(1);
    expect(screen.getByText("×2")).toBeTruthy();
    expect(container.textContent).toContain("failed-1");
    expect(container.textContent).toContain("failed-2");
  });
});

describe("toolActivityPreview", () => {
  it("bounds whitespace and redacts common inline secrets", () => {
    expect(toolActivityPreview({ command: "curl  --token=secret-value\nhttps://example.test" })).toBe("curl --token=•••• https://example.test");
    expect(toolActivityPreview({ path: "x".repeat(140) })).toHaveLength(118);
  });
});
