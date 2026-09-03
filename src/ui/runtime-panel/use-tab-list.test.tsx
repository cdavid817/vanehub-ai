// @vitest-environment jsdom

import { fireEvent, render, screen } from "@testing-library/react";
import { useState } from "react";
import { describe, expect, it } from "vitest";
import { useTabList } from "./use-tab-list";

const tabs = [{ id: "a" }, { id: "b" }, { id: "c" }];

/**
 * A minimal real `role="tablist"` built directly on the hook, exercised through real DOM events
 * rather than calling `handleKeyDown` with a hand-built fake event -- this is the same shape every
 * real consumer (13 of them as of this task) wires up, so a bug in that wiring contract (forgetting
 * `tabIndex`, mismatching the registered id) would show up here too, not just an algorithm-only test.
 */
function TestTabList({ initialActiveId, tabList = tabs }: { initialActiveId: string; tabList?: typeof tabs }) {
  const [activeId, setActiveId] = useState(initialActiveId);
  const { handleKeyDown, registerTabRef } = useTabList(tabList, activeId, setActiveId);
  return (
    <div onKeyDown={handleKeyDown} role="tablist">
      {tabList.map((tab) => (
        <button key={tab.id} ref={registerTabRef(tab.id)} role="tab" tabIndex={tab.id === activeId ? 0 : -1}>
          {tab.id}
        </button>
      ))}
    </div>
  );
}

describe("useTabList", () => {
  it("moves to the next tab on ArrowRight and moves real focus with it", () => {
    render(<TestTabList initialActiveId="a" />);
    const tabA = screen.getByRole("tab", { name: "a" });
    const tabB = screen.getByRole("tab", { name: "b" });
    tabA.focus();

    fireEvent.keyDown(tabA, { key: "ArrowRight" });

    expect(document.activeElement).toBe(tabB);
    expect(tabB.tabIndex).toBe(0);
    expect(tabA.tabIndex).toBe(-1);
  });

  it("wraps from the last tab back to the first on ArrowRight", () => {
    render(<TestTabList initialActiveId="c" />);
    const tabC = screen.getByRole("tab", { name: "c" });
    tabC.focus();

    fireEvent.keyDown(tabC, { key: "ArrowRight" });

    expect(document.activeElement).toBe(screen.getByRole("tab", { name: "a" }));
  });

  it("wraps from the first tab back to the last on ArrowLeft", () => {
    render(<TestTabList initialActiveId="a" />);
    const tabA = screen.getByRole("tab", { name: "a" });
    tabA.focus();

    fireEvent.keyDown(tabA, { key: "ArrowLeft" });

    expect(document.activeElement).toBe(screen.getByRole("tab", { name: "c" }));
  });

  it("jumps to the first tab on Home and the last tab on End", () => {
    render(<TestTabList initialActiveId="b" />);
    const tabB = screen.getByRole("tab", { name: "b" });
    tabB.focus();

    fireEvent.keyDown(tabB, { key: "Home" });
    expect(document.activeElement).toBe(screen.getByRole("tab", { name: "a" }));

    fireEvent.keyDown(screen.getByRole("tab", { name: "a" }), { key: "End" });
    expect(document.activeElement).toBe(screen.getByRole("tab", { name: "c" }));
  });

  it("ignores keys it does not handle, leaving focus and roving tabIndex unchanged", () => {
    render(<TestTabList initialActiveId="a" />);
    const tabA = screen.getByRole("tab", { name: "a" });
    tabA.focus();

    fireEvent.keyDown(tabA, { key: "Enter" });

    expect(document.activeElement).toBe(tabA);
    expect(tabA.tabIndex).toBe(0);
  });

  it("does nothing when there are no tabs", () => {
    render(<TestTabList initialActiveId="" tabList={[]} />);
    const tablist = screen.getByRole("tablist");

    expect(() => fireEvent.keyDown(tablist, { key: "ArrowRight" })).not.toThrow();
  });
});
