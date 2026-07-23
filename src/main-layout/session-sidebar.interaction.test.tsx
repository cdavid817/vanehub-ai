// @vitest-environment jsdom

import { fireEvent, screen } from "@testing-library/react";
import { useState } from "react";
import { beforeEach, describe, expect, it, vi } from "vitest";
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
    lifecycleState: "idle",
    folder: null,
    projectPath: null,
    worktreePath: null,
    worktreeName: null,
    worktreeBranch: null,
    remoteWorkspace: null,
    runtimeSessionId: null,
    categoryId: "source",
    pinned: false,
    archived: false,
    createdAt: "2026-07-23T00:00:00.000Z",
    updatedAt: "2026-07-23T00:00:00.000Z",
  };
}
