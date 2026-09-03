// @vitest-environment jsdom

import { fireEvent, render, screen, waitFor } from "@testing-library/react";
import { beforeEach, describe, expect, it, vi } from "vitest";
import "../i18n";

const mocks = vi.hoisted(() => ({
  loop: vi.fn(), work_item: vi.fn(), session: vi.fn(), run: vi.fn(),
}));

// Replaces the real registry so this file tests the picker's own orchestration (selection,
// confirm-before-link, kind switching, the raw-id fallback) without depending on
// agentService/workBoardService -- those are covered by execution-target-providers.test.ts.
vi.mock("./execution-target-providers", () => ({
  executionTargetSearchProviders: { loop: mocks.loop, work_item: mocks.work_item, session: mocks.session, run: mocks.run },
}));

import { ExecutionTargetPicker } from "./execution-target-picker";

// Labels/strings below are zh-CN: this codebase's test harness defaults to zh-CN, not English
// (goal-center.test.tsx's own established convention -- see e.g. its "验收"/"待验收" assertions).
function option(overrides: Partial<{ id: string; title: string; projectPath: string | null }> = {}) {
  return {
    id: "loop-1", title: "Fix auth loop", projectPath: "D:\\code\\vanehub",
    statusKey: "loops.definition.enabled", statusTone: "success" as const,
    ...overrides,
  };
}

describe("ExecutionTargetPicker", () => {
  const onLink = vi.fn();

  beforeEach(() => {
    onLink.mockReset();
    mocks.loop.mockReset().mockResolvedValue([]);
    mocks.work_item.mockReset().mockResolvedValue([]);
    mocks.session.mockReset().mockResolvedValue([]);
    mocks.run.mockReset().mockResolvedValue([]);
  });

  function renderPicker(pending = false) {
    render(<ExecutionTargetPicker onLink={onLink} pending={pending} />);
  }

  it("defaults to searching the loop kind on mount", async () => {
    renderPicker();
    await waitFor(() => expect(mocks.loop).toHaveBeenCalled());
  });

  it("shows a distinct empty state before and after typing a non-matching query", async () => {
    renderPicker();
    expect(await screen.findByText("输入关键字开始搜索。")).toBeTruthy();

    fireEvent.change(screen.getByLabelText("搜索关联对象"), { target: { value: "zzz" } });
    await waitFor(() => expect(screen.getByText("没有匹配“zzz”的结果。")).toBeTruthy());
  });

  it("shows a matched result's title, project, and status before it is linked", async () => {
    mocks.loop.mockResolvedValue([option()]);
    renderPicker();
    expect(await screen.findByText("Fix auth loop")).toBeTruthy();
    expect(screen.getByText("D:\\code\\vanehub")).toBeTruthy();
    expect(screen.getByText("已启用")).toBeTruthy();
  });

  it("stages a clicked result in a confirm panel instead of linking immediately", async () => {
    mocks.loop.mockResolvedValue([option()]);
    renderPicker();
    fireEvent.click(await screen.findByText("Fix auth loop"));

    expect(onLink).not.toHaveBeenCalled();
    expect(screen.getByRole("button", { name: "关联" })).toBeTruthy();
    expect(screen.getByRole("button", { name: "更换" })).toBeTruthy();
  });

  it("links the staged target only once Link is pressed, then returns to search", async () => {
    mocks.loop.mockResolvedValue([option()]);
    renderPicker();
    fireEvent.click(await screen.findByText("Fix auth loop"));

    fireEvent.click(screen.getByRole("button", { name: "关联" }));

    expect(onLink).toHaveBeenCalledWith("loop", "loop-1");
    expect(screen.queryByRole("button", { name: "更换" })).toBeNull();
  });

  it("returns to search without linking when Change is pressed", async () => {
    mocks.loop.mockResolvedValue([option()]);
    renderPicker();
    fireEvent.click(await screen.findByText("Fix auth loop"));

    fireEvent.click(screen.getByRole("button", { name: "更换" }));

    expect(onLink).not.toHaveBeenCalled();
    expect(screen.queryByRole("button", { name: "更换" })).toBeNull();
  });

  it("clears the staged selection and re-searches when the kind changes", async () => {
    mocks.loop.mockResolvedValue([option()]);
    renderPicker();
    fireEvent.click(await screen.findByText("Fix auth loop"));

    fireEvent.change(screen.getByLabelText("类型"), { target: { value: "run" } });

    expect(screen.queryByRole("button", { name: "更换" })).toBeNull();
    await waitFor(() => expect(mocks.run).toHaveBeenCalled());
  });

  it("keeps the raw-id path collapsed until explicitly opened", () => {
    renderPicker();
    expect(screen.queryByLabelText("目标 ID")).toBeNull();
  });

  it("links through the raw-id path once opened and filled in", async () => {
    renderPicker();
    fireEvent.click(screen.getByRole("button", { name: "手动输入 ID" }));
    fireEvent.change(screen.getByLabelText("目标 ID"), { target: { value: " loop-9 " } });

    fireEvent.click(screen.getByRole("button", { name: "关联" }));

    await waitFor(() => expect(onLink).toHaveBeenCalledWith("loop", "loop-9"));
  });

  it("rejects a raw-id submission left empty", async () => {
    renderPicker();
    fireEvent.click(screen.getByRole("button", { name: "手动输入 ID" }));

    fireEvent.click(screen.getByRole("button", { name: "关联" }));
    await new Promise((resolve) => setTimeout(resolve, 50));

    expect(onLink).not.toHaveBeenCalled();
  });

  it("disables the kind select, search input, and the confirm panel's Link button while pending", async () => {
    mocks.loop.mockResolvedValue([option()]);
    renderPicker(true);
    fireEvent.click(await screen.findByText("Fix auth loop"));

    expect((screen.getByLabelText("类型") as HTMLSelectElement).disabled).toBe(true);
    expect((screen.getByLabelText("搜索关联对象") as HTMLInputElement).disabled).toBe(true);
    expect((screen.getByRole("button", { name: "关联" }) as HTMLButtonElement).disabled).toBe(true);
  });
});
