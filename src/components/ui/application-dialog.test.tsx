// @vitest-environment jsdom

import { readFileSync, readdirSync, statSync } from "node:fs";
import { join } from "node:path";
import { cleanup, fireEvent, render, screen } from "@testing-library/react";
import { afterEach, describe, expect, it, vi } from "vitest";
import { ApplicationDialog } from "./application-dialog";

afterEach(cleanup);

function sourceFiles(directory: string): string[] {
  return readdirSync(directory).flatMap((entry) => {
    const path = join(directory, entry);
    if (statSync(path).isDirectory()) return sourceFiles(path);
    return /\.tsx?$/.test(entry) ? [path] : [];
  });
}

describe("ApplicationDialog", () => {
  it("closes on Escape and returns focus to the invoking control", () => {
    const onClose = vi.fn();
    const opener = document.createElement("button");
    document.body.append(opener);
    opener.focus();

    const view = render(
      <ApplicationDialog onClose={onClose} title="Title">
        <button type="button">Inside</button>
      </ApplicationDialog>,
    );

    fireEvent.keyDown(document, { key: "Escape" });
    expect(onClose).toHaveBeenCalledOnce();

    view.unmount();
    expect(document.activeElement).toBe(opener);
    opener.remove();
  });

  it("does not close while the caller reports blocking work", () => {
    const onClose = vi.fn();
    render(
      <ApplicationDialog closeDisabled onClose={onClose} title="Title">
        <button type="button">Inside</button>
      </ApplicationDialog>,
    );

    fireEvent.keyDown(document, { key: "Escape" });
    expect(onClose).not.toHaveBeenCalled();
  });

  it("wraps Tab focus at the first and last controls", () => {
    render(
      <ApplicationDialog onClose={vi.fn()} title="Title">
        <button type="button">First</button>
        <button type="button">Last</button>
      </ApplicationDialog>,
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

  it("moves initial focus to the designated control", () => {
    render(
      <ApplicationDialog onClose={vi.fn()} title="Title">
        <button type="button">Other</button>
        <button data-dialog-autofocus type="button">Preferred</button>
      </ApplicationDialog>,
    );
    expect(document.activeElement).toBe(screen.getByRole("button", { name: "Preferred" }));
  });

  it("renders a pinned footer only when one is supplied", () => {
    const withoutFooter = render(
      <ApplicationDialog onClose={vi.fn()} title="Title"><p>Body</p></ApplicationDialog>,
    );
    expect(screen.queryByText("Footer")).toBeNull();
    withoutFooter.unmount();

    render(
      <ApplicationDialog footer={<span>Footer</span>} onClose={vi.fn()} title="Title">
        <p>Body</p>
      </ApplicationDialog>,
    );
    expect(screen.getByText("Footer").textContent).toBe("Footer");
  });

  it("exposes the title and description through ARIA", () => {
    render(
      <ApplicationDialog description="Explains" onClose={vi.fn()} title="Title">
        <p>Body</p>
      </ApplicationDialog>,
    );
    const dialog = screen.getByRole("dialog");
    expect(dialog.getAttribute("aria-modal")).toBe("true");
    expect(dialog.getAttribute("aria-labelledby")).toBeTruthy();
    expect(dialog.getAttribute("aria-describedby")).toBeTruthy();
  });
});

/**
 * Hand-rolled modals kept reappearing next to a primitive that already handled dismissal and
 * focus correctly, and browser-native dialogs kept being reached for because they are one call.
 * Both now fail here rather than in review.
 */
describe("native browser dialogs", () => {
  it("are not called anywhere under src/", () => {
    const offenders = sourceFiles("src")
      .filter((path) => !path.includes("application-dialog.test"))
      .filter((path) => /window\.(prompt|alert|confirm)\s*\(/.test(readFileSync(path, "utf8")));
    expect(offenders).toEqual([]);
  });
});
