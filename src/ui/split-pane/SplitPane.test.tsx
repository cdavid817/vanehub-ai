// @vitest-environment jsdom

import { useState } from "react";
import { fireEvent, render, screen } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";
import { SplitPane, type SplitPaneProps } from "./SplitPane";

/** A controlled wrapper so keyboard steps accumulate the way a real caller's state would. */
function ControlledSplitPane(props: Omit<SplitPaneProps, "size" | "onSizeChange"> & { initialSize: number; onSizeChange?: (size: number) => void }) {
  const [size, setSize] = useState(props.initialSize);
  return (
    <SplitPane
      {...props}
      onSizeChange={(next) => { setSize(next); props.onSizeChange?.(next); }}
      size={size}
    />
  );
}

describe("SplitPane", () => {
  it("renders primary in the start position and secondary in the end position", () => {
    const { container } = render(
      <SplitPane
        direction="row"
        gutterLabel="Resize navigation"
        max={400}
        min={160}
        onSizeChange={vi.fn()}
        primary={<p>Navigation</p>}
        secondary={<p>Main</p>}
        size={240}
      />,
    );
    const texts = [...container.querySelectorAll("p")].map((el) => el.textContent);
    expect(texts).toEqual(["Navigation", "Main"]);
  });

  it("clamps an out-of-range size prop to the min/max bounds", () => {
    render(
      <SplitPane
        direction="row"
        gutterLabel="Resize navigation"
        max={400}
        min={160}
        onSizeChange={vi.fn()}
        primary={<p>Navigation</p>}
        secondary={<p>Main</p>}
        size={9999}
      />,
    );
    expect(screen.getByRole("separator").getAttribute("aria-valuenow")).toBe("400");
  });

  it("resizes via pointer drag and commits the final value on pointer up", () => {
    const onSizeChange = vi.fn();
    const onResizeEnd = vi.fn();
    render(
      <SplitPane
        direction="row"
        gutterLabel="Resize navigation"
        max={400}
        min={160}
        onResizeEnd={onResizeEnd}
        onSizeChange={onSizeChange}
        primary={<p>Navigation</p>}
        secondary={<p>Main</p>}
        size={240}
      />,
    );
    const gutter = screen.getByRole("separator");
    fireEvent.pointerDown(gutter, { clientX: 100 });
    fireEvent.pointerMove(document, { clientX: 140 });
    expect(onSizeChange).toHaveBeenLastCalledWith(280);
    fireEvent.pointerUp(document, { clientX: 150 });
    expect(onResizeEnd).toHaveBeenCalledWith(290);
  });

  it("resizes via arrow keys and jumps to bounds via Home/End", () => {
    const onSizeChange = vi.fn();
    render(
      <ControlledSplitPane
        direction="row"
        gutterLabel="Resize navigation"
        initialSize={240}
        max={400}
        min={160}
        onSizeChange={onSizeChange}
        primary={<p>Navigation</p>}
        secondary={<p>Main</p>}
      />,
    );
    const gutter = screen.getByRole("separator");
    fireEvent.keyDown(gutter, { key: "ArrowRight" });
    expect(onSizeChange).toHaveBeenLastCalledWith(256);
    fireEvent.keyDown(gutter, { key: "ArrowLeft", shiftKey: true });
    expect(onSizeChange).toHaveBeenLastCalledWith(192);
    fireEvent.keyDown(gutter, { key: "End" });
    expect(onSizeChange).toHaveBeenLastCalledWith(400);
    fireEvent.keyDown(gutter, { key: "Home" });
    expect(onSizeChange).toHaveBeenLastCalledWith(160);
  });

  it("flips which side is pixel-sized when resizedPane is secondary, without moving primary out of the start position", () => {
    const { container } = render(
      <SplitPane
        direction="row"
        gutterLabel="Resize inspector"
        max={400}
        min={160}
        onSizeChange={vi.fn()}
        primary={<p>Main</p>}
        resizedPane="secondary"
        secondary={<p>Inspector</p>}
        size={240}
      />,
    );
    const texts = [...container.querySelectorAll("p")].map((el) => el.textContent);
    expect(texts).toEqual(["Main", "Inspector"]);
    const inspectorWrapper = screen.getByText("Inspector").parentElement;
    expect(inspectorWrapper?.getAttribute("style")).toContain("width: 240px");
  });

  it("exposes the separator role with orientation and value range for assistive tech", () => {
    render(
      <SplitPane
        direction="column"
        gutterLabel="Resize runtime panel"
        max={480}
        min={120}
        onSizeChange={vi.fn()}
        primary={<p>Work surface</p>}
        secondary={<p>Runtime panel</p>}
        size={200}
      />,
    );
    const gutter = screen.getByRole("separator", { name: "Resize runtime panel" });
    expect(gutter.getAttribute("aria-orientation")).toBe("horizontal");
    expect(gutter.getAttribute("aria-valuemin")).toBe("120");
    expect(gutter.getAttribute("aria-valuemax")).toBe("480");
  });
});
