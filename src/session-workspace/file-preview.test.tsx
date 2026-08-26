/** @vitest-environment jsdom */
import { fireEvent, screen } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";
import { renderWithAppProviders } from "../test/render";
import type { FileContent } from "../types/session-workspace";
import { FilePreview } from "./file-preview";

function file(overrides: Partial<FileContent> = {}): FileContent {
  return {
    path: "src/main.rs",
    name: "main.rs",
    status: "text",
    size: 42,
    content: "fn main() {\n    let needle = 1;\n    println!(\"needle\");\n}\n",
    encoding: "utf-8",
    newline: "lf",
    ...overrides,
  };
}

function render(overrides: Partial<FileContent> = {}, props: { targetLine?: number | null } = {}) {
  return renderWithAppProviders(
    <FilePreview
      file={file(overrides)}
      status={{ kind: "current" }}
      targetLine={props.targetLine ?? null}
    />,
  );
}

function findBox() {
  return screen.getByRole("textbox", { name: /Find in file|在文件中查找/ });
}

function lineBox() {
  return screen.getByRole("textbox", { name: /^Line$|^行号$/ });
}

describe("FilePreview", () => {
  it("numbers every line", () => {
    render();

    // Four lines of content plus the empty one after the trailing newline: the count is what the
    // file actually contains, not what looks tidy.
    expect(screen.getByTestId("preview-line-1")).toBeTruthy();
    expect(screen.getByTestId("preview-line-4")).toBeTruthy();
  });

  it("reports the encoding and line endings a reader cannot see", () => {
    render({ encoding: "utf-8-bom", newline: "mixed" });

    // Both are invisible in the content and both change what a reader does: a BOM breaks shell
    // scripts and JSON parsers, mixed endings turn an ordinary edit into a whole-file diff.
    expect(screen.getByText(/BOM/)).toBeTruthy();
    expect(screen.getByText(/Mixed line endings|混合换行符/)).toBeTruthy();
  });

  it("says nothing about encoding for a file it never decoded", () => {
    render({ status: "binary", content: null, encoding: undefined, newline: undefined });

    // An "unknown" chip would be a row a reader learns to ignore. Absent is the honest rendering
    // of a fact this application never established.
    expect(screen.queryByText(/UTF-8/)).toBeNull();
  });

  it("counts the matches for a find query", () => {
    render();

    fireEvent.change(findBox(), { target: { value: "needle" } });

    // Two lines contain it. A count that showed occurrences rather than lines would disagree with
    // the number of places the arrows can take you.
    expect(screen.getByText("1/2")).toBeTruthy();
  });

  it("distinguishes a query that found nothing from one that has not run", () => {
    render();

    fireEvent.change(findBox(), { target: { value: "absent" } });

    expect(screen.getByText("0")).toBeTruthy();
  });

  it("wraps around the end of the matches", () => {
    render();
    fireEvent.change(findBox(), { target: { value: "needle" } });

    fireEvent.keyDown(findBox(), { key: "Enter" });
    expect(screen.getByText("2/2")).toBeTruthy();

    fireEvent.keyDown(findBox(), { key: "Enter" });
    // A find walks one file repeatedly. Stopping at the last match would make a reader scroll back
    // to the top by hand every time.
    expect(screen.getByText("1/2")).toBeTruthy();
  });

  it("steps backwards with shift", () => {
    render();
    fireEvent.change(findBox(), { target: { value: "needle" } });

    fireEvent.keyDown(findBox(), { key: "Enter", shiftKey: true });

    // What every find box does. A reader who has learned one should not have to learn this one.
    expect(screen.getByText("2/2")).toBeTruthy();
  });

  it("selects the line a reader asks for", () => {
    render();

    fireEvent.change(lineBox(), { target: { value: "3" } });
    fireEvent.keyDown(lineBox(), { key: "Enter" });

    expect(screen.getByTestId("preview-line-3").getAttribute("data-selected")).toBe("true");
  });

  it("clamps a line past the end rather than refusing it", () => {
    render();

    fireEvent.change(lineBox(), { target: { value: "9999" } });
    fireEvent.keyDown(lineBox(), { key: "Enter" });

    // A reader who types 9999 in a short file meant "the end". An error would answer a question
    // they did not think they were asking.
    expect(screen.getByTestId("preview-line-5").getAttribute("data-selected")).toBe("true");
  });

  it("opens on the line a content-search result named", () => {
    render({}, { targetLine: 2 });

    expect(screen.getByTestId("preview-line-2").getAttribute("data-selected")).toBe("true");
  });

  it("hands the evidence action the file it is showing", () => {
    const onShowEvidence = vi.fn();
    renderWithAppProviders(
      <FilePreview
        file={file()}
        onShowEvidence={onShowEvidence}
        status={{ kind: "current" }}
        targetLine={null}
      />,
    );

    fireEvent.click(screen.getByRole("button", { name: /records for this file|此文件的记录/ }));

    expect(onShowEvidence).toHaveBeenCalledWith("src/main.rs");
  });

  it("omits the evidence action where nothing can act on it", () => {
    render();

    // Absent rather than inert: a button that does nothing is a worse answer than no button.
    expect(
      screen.queryByRole("button", { name: /records for this file|此文件的记录/ }),
    ).toBeNull();
  });
});
