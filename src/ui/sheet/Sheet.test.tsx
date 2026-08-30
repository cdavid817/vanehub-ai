// @vitest-environment jsdom

import { cleanup, fireEvent, render, screen } from "@testing-library/react";
import { afterEach, describe, expect, it, vi } from "vitest";
import { Sheet } from "./Sheet";

afterEach(cleanup);

describe("Sheet", () => {
  it("closes on Escape and returns focus to the invoking control", () => {
    const onClose = vi.fn();
    const opener = document.createElement("button");
    document.body.append(opener);
    opener.focus();

    const view = render(
      <Sheet onClose={onClose} placement="right" title="Session Overview">
        <button type="button">Inside</button>
      </Sheet>,
    );

    fireEvent.keyDown(document, { key: "Escape" });
    expect(onClose).toHaveBeenCalledOnce();

    view.unmount();
    expect(document.activeElement).toBe(opener);
    opener.remove();
  });

  it("does not close on Escape while closeDisabled explains why", () => {
    const onClose = vi.fn();
    render(
      <Sheet closeDisabled onClose={onClose} placement="right" title="Session Overview">
        <button type="button">Inside</button>
      </Sheet>,
    );
    fireEvent.keyDown(document, { key: "Escape" });
    expect(onClose).not.toHaveBeenCalled();
  });

  it("closes on backdrop click unless closeDisabled", () => {
    const onClose = vi.fn();
    const { container } = render(
      <Sheet onClose={onClose} placement="right" title="Session Overview">
        <p>Body</p>
      </Sheet>,
    );
    fireEvent.mouseDown(container.querySelector('[role="presentation"]') as Element);
    expect(onClose).toHaveBeenCalledOnce();
  });

  it("wraps Tab focus at the first and last controls", () => {
    render(
      <Sheet onClose={vi.fn()} placement="bottom" title="Filters">
        <button type="button">First</button>
        <button type="button">Last</button>
      </Sheet>,
    );
    const first = screen.getByRole("button", { name: "First" });
    const last = screen.getByRole("button", { name: "Last" });

    last.focus();
    fireEvent.keyDown(document, { key: "Tab" });
    expect(document.activeElement).toBe(first);

    first.focus();
    fireEvent.keyDown(document, { key: "Tab", shiftKey: true });
    expect(document.activeElement).toBe(last);
  });

  it("exposes the title and description through ARIA as a modal dialog", () => {
    render(
      <Sheet description="Read-only summary" onClose={vi.fn()} placement="full" title="Session Overview">
        <p>Body</p>
      </Sheet>,
    );
    const dialog = screen.getByRole("dialog");
    expect(dialog.getAttribute("aria-modal")).toBe("true");
    expect(dialog.getAttribute("aria-labelledby")).toBeTruthy();
    expect(dialog.getAttribute("aria-describedby")).toBeTruthy();
  });

  it("renders a pinned footer only when one is supplied", () => {
    const withoutFooter = render(<Sheet onClose={vi.fn()} placement="left" title="Navigation"><p>Body</p></Sheet>);
    expect(screen.queryByText("Footer")).toBeNull();
    withoutFooter.unmount();

    render(
      <Sheet footer={<span>Footer</span>} onClose={vi.fn()} placement="left" title="Navigation">
        <p>Body</p>
      </Sheet>,
    );
    expect(screen.getByText("Footer").textContent).toBe("Footer");
  });

  it.each(["left", "right", "bottom", "full"] as const)("positions the %s placement at its edge", (placement) => {
    render(<Sheet onClose={vi.fn()} placement={placement} title="Panel"><p>Body</p></Sheet>);
    const dialog = screen.getByRole("dialog");
    if (placement === "left") expect(dialog.className).toContain("left-0");
    if (placement === "right") expect(dialog.className).toContain("right-0");
    if (placement === "bottom") expect(dialog.className).toContain("bottom-0");
    if (placement === "full") expect(dialog.className).toContain("inset-0");
  });
});
