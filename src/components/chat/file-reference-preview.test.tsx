// @vitest-environment jsdom

import { fireEvent, render, screen, waitFor } from "@testing-library/react";
import { beforeEach, describe, expect, it, vi } from "vitest";
import "../../i18n";
import type { FileContent } from "../../types/session-workspace";

const { readSessionFile } = vi.hoisted(() => ({ readSessionFile: vi.fn() }));
vi.mock("../../services/runtime-agent-client", () => ({ agentService: { readSessionFile } }));

// The line list is virtualized, and the virtualizer measures a scroll container jsdom
// reports as zero-sized, so it would render an empty window and make every assertion here
// pass against nothing. Rendering all rows keeps these tests on what this change owns —
// selection and attachment. That windowing itself works is covered by the manual check,
// and that a selection survives rows leaving the window is covered by line-selection
// tests, which is exactly why the selection is kept keyed by line number rather than
// held on a rendered row.
vi.mock("../measured-virtual-list", () => ({
  MeasuredVirtualList: <T,>({ items, renderItem, testId }: { items: readonly T[]; renderItem: (item: T, index: number) => unknown; testId?: string }) => (
    <div data-testid={testId}>{items.map((item, index) => renderItem(item, index) as never)}</div>
  ),
}));

import { FileReferencePreviewDialog } from "./FileReferencePreviewDialog";

const content = Array.from({ length: 6 }, (_, index) => `let line${index + 1} = ${index + 1};`).join("\n");

function fileResult(overrides: Partial<FileContent> = {}): FileContent {
  return { path: "src/utils.rs", name: "utils.rs", status: "text", size: content.length, content, ...overrides };
}

function renderDialog(onAttach = vi.fn(), onCancel = vi.fn()) {
  render(
    <FileReferencePreviewDialog
      name="utils.rs"
      onAttach={onAttach}
      onCancel={onCancel}
      path="src/utils.rs"
      sessionId="session-1"
    />,
  );
  return { onAttach, onCancel };
}

describe("file reference preview", () => {
  beforeEach(() => {
    readSessionFile.mockReset();
    readSessionFile.mockResolvedValue(fileResult());
  });

  it("renders the file with 1-based line numbers", async () => {
    renderDialog();
    await waitFor(() => expect(screen.getByTestId("preview-line-1")).toBeTruthy());
    expect(screen.getByTestId("preview-line-1").textContent).toContain("1");
    expect(screen.getByTestId("preview-line-6")).toBeTruthy();
    expect(screen.queryByTestId("preview-line-0")).toBeNull();
  });

  it("attaches the selected range and marks the selected lines", async () => {
    const { onAttach } = renderDialog();
    await waitFor(() => expect(screen.getByTestId("preview-line-2")).toBeTruthy());
    fireEvent.click(screen.getByTestId("preview-line-2"));
    fireEvent.click(screen.getByTestId("preview-line-4"));
    expect(screen.getByTestId("preview-line-3").getAttribute("data-selected")).toBe("true");
    expect(screen.getByTestId("preview-line-5").getAttribute("data-selected")).toBeNull();
    fireEvent.click(screen.getByText("引用所选范围"));
    expect(onAttach).toHaveBeenCalledWith({ startLine: 2, endLine: 4 });
  });

  it("selects the same range when clicked bottom-up", async () => {
    const { onAttach } = renderDialog();
    await waitFor(() => expect(screen.getByTestId("preview-line-5")).toBeTruthy());
    fireEvent.click(screen.getByTestId("preview-line-5"));
    fireEvent.click(screen.getByTestId("preview-line-2"));
    fireEvent.click(screen.getByText("引用所选范围"));
    expect(onAttach).toHaveBeenCalledWith({ startLine: 2, endLine: 5 });
  });

  it("attaches a one-line range from a single click", async () => {
    const { onAttach } = renderDialog();
    await waitFor(() => expect(screen.getByTestId("preview-line-3")).toBeTruthy());
    fireEvent.click(screen.getByTestId("preview-line-3"));
    fireEvent.click(screen.getByText("引用所选范围"));
    expect(onAttach).toHaveBeenCalledWith({ startLine: 3, endLine: 3 });
  });

  it("restarts the selection on a third click", async () => {
    const { onAttach } = renderDialog();
    await waitFor(() => expect(screen.getByTestId("preview-line-1")).toBeTruthy());
    fireEvent.click(screen.getByTestId("preview-line-1"));
    fireEvent.click(screen.getByTestId("preview-line-4"));
    fireEvent.click(screen.getByTestId("preview-line-6"));
    expect(screen.getByTestId("preview-line-4").getAttribute("data-selected")).toBeNull();
    fireEvent.click(screen.getByText("引用所选范围"));
    expect(onAttach).toHaveBeenCalledWith({ startLine: 6, endLine: 6 });
  });

  it("attaches no range for the whole file", async () => {
    const { onAttach } = renderDialog();
    await waitFor(() => expect(screen.getByTestId("preview-line-1")).toBeTruthy());
    fireEvent.click(screen.getByText("引用整个文件"));
    expect(onAttach).toHaveBeenCalledWith({});
  });

  it("disables the selection action until a line is picked", async () => {
    renderDialog();
    await waitFor(() => expect(screen.getByTestId("preview-line-1")).toBeTruthy());
    expect(screen.getByText("引用所选范围").closest("button")?.disabled).toBe(true);
    fireEvent.click(screen.getByTestId("preview-line-1"));
    expect(screen.getByText("引用所选范围").closest("button")?.disabled).toBe(false);
  });

  it("offers only the whole-file action for a binary or oversized file", async () => {
    readSessionFile.mockResolvedValue(fileResult({ status: "binary", content: null }));
    renderDialog();
    await waitFor(() => expect(screen.getByText("该文件是二进制文件，无法预览。")).toBeTruthy());
    expect(screen.queryByTestId("preview-line-1")).toBeNull();
    expect(screen.getByText("引用整个文件").closest("button")?.disabled).toBe(false);
    expect(screen.getByText("引用所选范围").closest("button")?.disabled).toBe(true);
  });

  it("offers nothing for a missing file", async () => {
    readSessionFile.mockResolvedValue(fileResult({ status: "missing", content: null }));
    renderDialog();
    await waitFor(() => expect(screen.getByText("该文件不可用。")).toBeTruthy());
    expect(screen.getByText("引用整个文件").closest("button")?.disabled).toBe(true);
  });

  it("reports a failed read without offering an attach", async () => {
    readSessionFile.mockRejectedValue(new Error("boom"));
    renderDialog();
    await waitFor(() => expect(screen.getByText("该文件不可用。")).toBeTruthy());
    expect(screen.getByText("引用整个文件").closest("button")?.disabled).toBe(true);
  });

  it("cancels without attaching", async () => {
    const { onAttach, onCancel } = renderDialog();
    await waitFor(() => expect(screen.getByTestId("preview-line-1")).toBeTruthy());
    fireEvent.click(screen.getByText("取消"));
    expect(onCancel).toHaveBeenCalled();
    expect(onAttach).not.toHaveBeenCalled();
  });
});
