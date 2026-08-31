// @vitest-environment jsdom

import { act, fireEvent, screen } from "@testing-library/react";
import { useState } from "react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import type { Session, SessionCategory } from "../types/agent";
import { renderWithAppProviders } from "../test/render";
import { SessionSidebar } from "./session-sidebar";

const categories: SessionCategory[] = [
  { id: "source", name: "Source", sortOrder: 0, createdAt: "2026-07-23T00:00:00.000Z", updatedAt: "2026-07-23T00:00:00.000Z" },
  { id: "target", name: "Target", sortOrder: 1, createdAt: "2026-07-23T00:00:00.000Z", updatedAt: "2026-07-23T00:00:00.000Z" },
];

describe("SessionSidebar category interactions", () => {
  beforeEach(() => {
    localStorage.clear();
  });

  // A leaked fake-timer state from a failed assertion mid-test would hang every subsequent test's
  // own real-timer waits.
  afterEach(() => vi.useRealTimers());

  it("assigns a dragged Session once and presents it in the target category", async () => {
    const assigned = vi.fn();
    const { user } = renderWithAppProviders(<SidebarHarness onAssigned={assigned} />);
    await user.click(screen.getByRole("button", { name: /^分类$/ }));
    await user.click(screen.getByRole("button", { name: /^Source/ }));

    const transfer = createDataTransfer();
    fireEvent.dragStart(screen.getByRole("button", { name: /Drag session/ }), { dataTransfer: transfer });
    fireEvent.drop(categorySection("target"), { dataTransfer: transfer });

    expect(assigned).toHaveBeenCalledOnce();
    expect(assigned).toHaveBeenCalledWith(expect.objectContaining({ id: "session-1" }), "target");
    await user.click(screen.getByRole("button", { name: /^Target/ }));
    expect(screen.getByRole("button", { name: /Drag session/ })).toBeTruthy();
  });

  it("ignores an invalid drag id and preserves the existing assignment", async () => {
    const assigned = vi.fn();
    const { user } = renderWithAppProviders(<SidebarHarness onAssigned={assigned} />);
    await user.click(screen.getByRole("button", { name: /^分类$/ }));
    await user.click(screen.getByRole("button", { name: /^Source/ }));

    const transfer = createDataTransfer();
    transfer.setData("text/plain", "missing-session");
    fireEvent.drop(categorySection("target"), { dataTransfer: transfer });

    expect(assigned).not.toHaveBeenCalled();
    expect(screen.getByRole("button", { name: /Drag session/ })).toBeTruthy();
  });

  // 7.15
  it("highlights the drop target while a drag is over it, and clears the highlight once it leaves", async () => {
    const { user } = renderWithAppProviders(<SidebarHarness onAssigned={vi.fn()} />);
    await user.click(screen.getByRole("button", { name: /^分类$/ }));
    await user.click(screen.getByRole("button", { name: /^Source/ }));
    await user.click(screen.getByRole("button", { name: /^Target/ }));

    const target = categorySection("target");
    expect(target.className).not.toContain("ring-primary/50");

    fireEvent.dragEnter(target);
    expect(target.className).toContain("ring-primary/50");

    // Fired on the section itself (event.target === event.currentTarget), matching a drag that
    // leaves the section entirely rather than moving between its own children.
    fireEvent.dragLeave(target, { target });
    expect(target.className).not.toContain("ring-primary/50");
  });

  it("flashes success feedback on the section a drop lands in, then clears it", async () => {
    const { user } = renderWithAppProviders(<SidebarHarness onAssigned={vi.fn()} />);
    await user.click(screen.getByRole("button", { name: /^分类$/ }));
    await user.click(screen.getByRole("button", { name: /^Source/ }));
    await user.click(screen.getByRole("button", { name: /^Target/ }));

    // Fake timers only from here: `userEvent`'s own clicks above need real ones, and switching
    // mid-test (rather than never mixing at all) is safe as long as nothing async spans the swap.
    vi.useFakeTimers();
    const transfer = createDataTransfer();
    fireEvent.dragStart(screen.getByRole("button", { name: /Drag session/ }), { dataTransfer: transfer });
    const target = categorySection("target");
    fireEvent.drop(target, { dataTransfer: transfer });

    expect(target.className).toContain("success");
    // `act()`: the state update this timer triggers happens outside any RTL-dispatched event, so
    // nothing else flushes it before the assertion below reads the DOM.
    act(() => { vi.advanceTimersByTime(600); });
    expect(categorySection("target").className).not.toContain("success");
    vi.useRealTimers();
  });
});

function SidebarHarness({ onAssigned }: { onAssigned: (session: Session, categoryId: string | null) => void }) {
  const [sessions, setSessions] = useState([session()]);
  return (
    <SessionSidebar
      activeSessionId="session-1"
      agentsAvailable
      archivedSessions={[]}
      categories={categories}
      onAssignCategory={(selected, categoryId) => {
        onAssigned(selected, categoryId);
        setSessions((current) => current.map((item) => item.id === selected.id ? { ...item, categoryId } : item));
      }}
      onBatchDelete={vi.fn()}
      onContextMenu={vi.fn()}
      onNew={vi.fn()}
      onSearchChange={vi.fn()}
      onSelect={vi.fn()}
      searchQuery=""
      searchResults={[]}
      sessions={sessions}
    />
  );
}

function categorySection(id: string) {
  const section = document.querySelector<HTMLElement>(`[data-session-category-id="${id}"]`);
  if (!section) throw new Error(`Missing category section: ${id}`);
  return section;
}

function createDataTransfer(): DataTransfer {
  const values = new Map<string, string>();
  return {
    clearData: (format?: string) => format ? values.delete(format) : values.clear(),
    dropEffect: "move",
    effectAllowed: "all",
    files: [] as unknown as FileList,
    getData: (format: string) => values.get(format) ?? "",
    items: [] as unknown as DataTransferItemList,
    setData: (format: string, value: string) => {
      values.set(format, value);
    },
    setDragImage: () => undefined,
    types: [],
  };
}

function session(): Session {
  return {
    id: "session-1",
    title: "Drag session",
    agentId: "codex-cli",
    interactionMode: "cli",
    personalizationMode: "standard", lifecycleState: "idle",
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
    categoryId: "source",
    pinned: false,
    archived: false,
    createdAt: "2026-07-23T00:00:00.000Z",
    updatedAt: "2026-07-23T00:00:00.000Z",
  };
}
