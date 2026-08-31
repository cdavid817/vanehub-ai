// @vitest-environment jsdom

import { forwardRef, useImperativeHandle } from "react";
import { render, screen } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";
import type { Session } from "../types/agent";
import { SESSION_LIST_VIRTUALIZE_THRESHOLD, SessionRowList } from "./session-row-list";

// `@tanstack/react-virtual` measures against a real layout (clientHeight etc.), which jsdom never
// provides — every item's virtual bounding box comes back zero-height, so a real VirtualList
// renders zero rows here regardless of `items.length`. Replaced with a component that honors the
// same renderItem-per-item and imperative-handle (`scrollToIndex`) contract without the real
// measurement machinery, matching the fix this codebase already uses for `EntityList`'s own tests.
const scrollToIndex = vi.fn();
vi.mock("../ui/virtual-list/VirtualList", () => ({
  VirtualList: forwardRef(({ ariaLabel, getItemKey, items, renderItem }: {
    ariaLabel: string; getItemKey: (item: unknown, index: number) => string;
    items: readonly unknown[]; renderItem: (item: unknown, index: number) => React.ReactNode;
  }, ref: React.Ref<{ scrollToIndex: typeof scrollToIndex }>) => {
    useImperativeHandle(ref, () => ({ scrollToIndex }));
    return (
      <div aria-label={ariaLabel} data-testid="fake-virtual-list">
        {items.map((item, index) => <div key={getItemKey(item, index)}>{renderItem(item, index)}</div>)}
      </div>
    );
  }),
}));

function session(id: string): Session {
  return {
    id,
    personalizationMode: "standard",
    title: `Session ${id}`,
    agentId: "claude-code",
    interactionMode: "cli",
    lifecycleState: "idle",
    recoveryStatus: "clean",
    recoveryRevision: 0,
    stateRevision: 0,
    historyRevision: 0,
    activeExecutionRunId: null,
    folder: null,
    projectPath: null,
    worktreePath: null,
    worktreeName: null,
    worktreeBranch: null,
    remoteWorkspace: null,
    remoteSshConnectionId: null,
    remoteSshConnectionRevision: null,
    runtimeSessionId: null,
    categoryId: null,
    pinned: false,
    archived: false,
    createdAt: "2026-08-30T00:00:00.000Z",
    updatedAt: "2026-08-30T00:00:00.000Z",
  };
}

// `data-session-id` matters here, not just `data-testid`: it's the same attribute the real
// `SessionCard` carries and the one `SessionRowList`'s own scroll-anchor effect queries by.
const card = (item: Session) => <span data-session-id={item.id} data-testid={`row-${item.id}`}>{item.title}</span>;

describe("SessionRowList", () => {
  it("renders a plain, unvirtualized list below the threshold", () => {
    const sessions = [session("a"), session("b")];
    render(<SessionRowList activeSessionId={null} ariaLabel="Sessions" card={card} sessions={sessions} />);

    expect(screen.queryByTestId("fake-virtual-list")).toBeNull();
    expect(screen.getByTestId("row-a")).toBeTruthy();
    expect(screen.getByTestId("row-b")).toBeTruthy();
  });

  it("switches to the virtualized list at exactly the spec'd one-thousand-session threshold", () => {
    const sessions = Array.from({ length: SESSION_LIST_VIRTUALIZE_THRESHOLD }, (_, index) => session(String(index)));
    render(<SessionRowList activeSessionId={null} ariaLabel="Sessions" card={card} sessions={sessions} />);

    expect(screen.getByTestId("fake-virtual-list")).toBeTruthy();
    // Every item is still reachable through it (the fake renders them all; the real one would
    // window them, which is exactly what "virtualize" means) — this proves items/getItemKey/
    // renderItem were wired through correctly, not merely that some fallback rendered.
    expect(screen.getByTestId("row-0")).toBeTruthy();
    expect(screen.getByTestId(`row-${SESSION_LIST_VIRTUALIZE_THRESHOLD - 1}`)).toBeTruthy();
  });

  it("stays on the plain list one session short of the threshold", () => {
    const sessions = Array.from({ length: SESSION_LIST_VIRTUALIZE_THRESHOLD - 1 }, (_, index) => session(String(index)));
    render(<SessionRowList activeSessionId={null} ariaLabel="Sessions" card={card} sessions={sessions} />);

    expect(screen.queryByTestId("fake-virtual-list")).toBeNull();
  });

  it("uses the session id as the stable item key", () => {
    const sessions = Array.from({ length: SESSION_LIST_VIRTUALIZE_THRESHOLD }, (_, index) => session(`stable-${index}`));
    const { container } = render(<SessionRowList activeSessionId={null} ariaLabel="Sessions" card={card} sessions={sessions} />);

    expect(container.querySelector('[data-testid="row-stable-0"]')).toBeTruthy();
  });
});

describe("SessionRowList scroll anchor (7.11)", () => {
  it("scrolls the active session into view in the plain (unvirtualized) list", () => {
    const sessions = [session("a"), session("b")];
    const scrollIntoView = vi.fn();
    HTMLElement.prototype.scrollIntoView = scrollIntoView;

    render(<SessionRowList activeSessionId="b" ariaLabel="Sessions" card={card} sessions={sessions} />);

    expect(scrollIntoView).toHaveBeenCalledWith({ block: "nearest" });
  });

  it("does not scroll when there is no active session", () => {
    const sessions = [session("a"), session("b")];
    const scrollIntoView = vi.fn();
    HTMLElement.prototype.scrollIntoView = scrollIntoView;

    render(<SessionRowList activeSessionId={null} ariaLabel="Sessions" card={card} sessions={sessions} />);

    expect(scrollIntoView).not.toHaveBeenCalled();
  });

  it("scrolls the active session's index into view in the virtualized list", () => {
    scrollToIndex.mockClear();
    const sessions = Array.from({ length: SESSION_LIST_VIRTUALIZE_THRESHOLD }, (_, index) => session(String(index)));
    render(<SessionRowList activeSessionId="42" ariaLabel="Sessions" card={card} sessions={sessions} />);

    expect(scrollToIndex).toHaveBeenCalledWith(42, "auto");
  });
});
