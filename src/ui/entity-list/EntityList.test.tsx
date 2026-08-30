// @vitest-environment jsdom

import { Fragment, forwardRef, useImperativeHandle, useState } from "react";
import { fireEvent, render, screen } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";
import { EntityList } from "./EntityList";
import type { VirtualListProps } from "../virtual-list/VirtualList";

// jsdom has no real layout engine, so @tanstack/react-virtual's viewport-culled range is always
// empty (clientHeight reads 0) — this repo's established pattern (trace-waterfall.test.tsx) is to
// replace the virtualizer with a plain map over every item, bypassing windowing entirely.
vi.mock("../../components/measured-virtual-list", () => ({
  MeasuredVirtualList: forwardRef(function MockMeasuredVirtualList<T>(
    { activeDescendantId, ariaLabel, getItemKey, items, onKeyDown, renderItem, role = "list" }: VirtualListProps<T>,
    ref: React.ForwardedRef<unknown>,
  ) {
    useImperativeHandle(ref, () => ({ measure: () => {}, scrollToIndex: () => {}, scrollToStart: () => {} }));
    return (
      <div aria-activedescendant={activeDescendantId} aria-label={ariaLabel} onKeyDown={onKeyDown} role={role} tabIndex={0}>
        {items.map((item, index) => <Fragment key={getItemKey(item, index)}>{renderItem(item, index)}</Fragment>)}
      </div>
    );
  }),
}));

interface Session {
  id: string;
  title: string;
}

const SESSIONS: Session[] = [
  { id: "s1", title: "Fix flaky test" },
  { id: "s2", title: "Add dark mode" },
  { id: "s3", title: "Refactor auth" },
];

function ControlledEntityList({ onActivate }: { onActivate?: (item: Session) => void }) {
  const [activeId, setActiveId] = useState<string | undefined>(SESSIONS[0].id);
  return (
    <EntityList
      activeId={activeId}
      ariaLabel="Sessions"
      estimateSize={() => 32}
      itemKey={(item) => item.id}
      items={SESSIONS}
      onActivate={onActivate}
      onActiveIdChange={setActiveId}
      renderItem={(item, isActive) => <span>{item.title}{isActive ? " (active)" : ""}</span>}
    />
  );
}

describe("EntityList", () => {
  it("renders every item and marks the active one via role='option'/aria-selected", () => {
    render(<ControlledEntityList />);
    const options = screen.getAllByRole("option");
    expect(options.length).toBe(SESSIONS.length);
    expect(screen.getByText("Fix flaky test (active)")).toBeTruthy();
    expect(options[0].getAttribute("aria-selected")).toBe("true");
    expect(options[1].getAttribute("aria-selected")).toBe("false");
  });

  it("exposes the listbox/activedescendant pattern instead of per-item DOM focus", () => {
    render(<ControlledEntityList />);
    const listbox = screen.getByRole("listbox", { name: "Sessions" });
    expect(listbox.getAttribute("aria-activedescendant")).toBe("entity-list-option-s1");
    for (const option of screen.getAllByRole("option")) {
      expect(option.getAttribute("tabindex")).toBeNull();
    }
  });

  it("moves the active item with arrow keys and wraps at neither end", () => {
    render(<ControlledEntityList />);
    const listbox = screen.getByRole("listbox");
    fireEvent.keyDown(listbox, { key: "ArrowDown" });
    expect(screen.getByText("Add dark mode (active)")).toBeTruthy();
    fireEvent.keyDown(listbox, { key: "ArrowUp" });
    fireEvent.keyDown(listbox, { key: "ArrowUp" });
    expect(screen.getByText("Fix flaky test (active)")).toBeTruthy();
  });

  it("jumps to the first/last item with Home/End", () => {
    render(<ControlledEntityList />);
    const listbox = screen.getByRole("listbox");
    fireEvent.keyDown(listbox, { key: "End" });
    expect(screen.getByText("Refactor auth (active)")).toBeTruthy();
    fireEvent.keyDown(listbox, { key: "Home" });
    expect(screen.getByText("Fix flaky test (active)")).toBeTruthy();
  });

  it("activates the current item with Enter, and a clicked item with a click", () => {
    const onActivate = vi.fn();
    render(<ControlledEntityList onActivate={onActivate} />);
    fireEvent.keyDown(screen.getByRole("listbox"), { key: "Enter" });
    expect(onActivate).toHaveBeenCalledWith(SESSIONS[0]);

    fireEvent.click(screen.getByText("Add dark mode"));
    expect(onActivate).toHaveBeenCalledWith(SESSIONS[1]);
  });
});
