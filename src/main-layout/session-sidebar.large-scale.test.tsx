// @vitest-environment jsdom

import { render } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import "../i18n";
import { agentService } from "../services/runtime-agent-client";
import { generateSessions } from "../testing/fixtures/session-fixtures";
import { SESSION_LIST_VIRTUALIZE_THRESHOLD } from "./session-row-list";
import { SessionSidebar } from "./session-sidebar";

/**
 * 21.7: `session-row-list.test.tsx` already proves the threshold switch and id-based keying, but
 * its own `VirtualList` mock renders every item ("the fake renders them all; the real one would
 * window them" -- that file's own comment), by design, to prove wiring rather than DOM bounds. This
 * file instead mocks one level lower -- `@tanstack/react-virtual`'s own `useVirtualizer` -- so the
 * real `SessionSidebar` -> real `SessionRowList` -> real `VirtualList` (`MeasuredVirtualList`) path
 * runs and actually performs the `virtualItems.map(...)` windowing it exists to do, against a fake
 * but bounded measurement result standing in for jsdom's real but useless zero-clientHeight one
 * (a standing constraint of this test environment, not something worth working around here).
 */
const FAKE_VIRTUAL_WINDOW = 30; // Stands in for "visible rows + overscan" in a real viewport --
// comfortably below both this file's 1,000-session fixture and the real overscan=8 the component
// requests, so a bounded DOM count here can only come from the real windowing logic actually
// running, never from the fixture happening to be small enough to render in full.

interface FakeVirtualizerOptions {
  count: number;
  getItemKey: (index: number) => string;
}

vi.mock("@tanstack/react-virtual", () => ({
  useVirtualizer: (options: FakeVirtualizerOptions) => {
    const windowSize = Math.min(FAKE_VIRTUAL_WINDOW, options.count);
    const virtualItems = Array.from({ length: windowSize }, (_unused, index) => ({
      key: options.getItemKey(index),
      index,
      start: index * 60,
    }));
    return {
      getVirtualItems: () => virtualItems,
      getTotalSize: () => options.count * 60,
      measure: () => undefined,
      measureElement: () => undefined,
      scrollToIndex: () => undefined,
      scrollToOffset: () => undefined,
    };
  },
}));

function renderSidebarWith(sessions: ReturnType<typeof generateSessions>) {
  return render(
    <SessionSidebar
      activeSessionId={null}
      agentsAvailable
      archivedSessions={[]}
      categories={[]}
      onAssignCategory={vi.fn()}
      onBatchDelete={vi.fn()}
      onContextMenu={vi.fn()}
      onNew={vi.fn()}
      onSearchChange={vi.fn()}
      onSelect={vi.fn()}
      searchQuery=""
      searchResults={[]}
      sessions={sessions}
    />,
  );
}

beforeEach(() => localStorage.clear());
afterEach(() => vi.restoreAllMocks());

describe("SessionSidebar at 1,000-session scale (21.7)", () => {
  it("keeps rendered session rows bounded regardless of the underlying session count", () => {
    const sessions = generateSessions(SESSION_LIST_VIRTUALIZE_THRESHOLD);

    const { container } = renderSidebarWith(sessions);

    // `SessionCard`'s own root carries `data-session-id` (session-card.tsx) -- one per rendered
    // row, real or fake virtualizer alike, so this counts actual DOM rows, not virtual bookkeeping.
    const rendered = container.querySelectorAll("[data-session-id]");
    expect(rendered.length).toBe(FAKE_VIRTUAL_WINDOW);
    expect(rendered.length).toBeLessThan(sessions.length);
  });

  it("fetches nothing of its own while rendering 1,000 sessions -- the list stays purely prop-driven", () => {
    // `session-card.tsx` and `session-row-list.tsx` were both read in full for this task: neither
    // imports `agentService` (or any other service) at all -- rendering N rows cannot fan out into
    // N (or any) fetches, structurally, not just by observation. The real session fetch is the
    // single `useQuery(["sessions"], () => agentService.listSessions())` in
    // `use-main-layout-model.ts`, one level above this component, out of `SessionSidebar`'s own
    // reach entirely. This test turns that structural reading into a real, behavioral regression
    // guard: every method most plausibly involved in a future accidental per-row fetch stays
    // uncalled across a full 1,000-row render.
    const listSessions = vi.spyOn(agentService, "listSessions");
    const listArchivedSessions = vi.spyOn(agentService, "listArchivedSessions");
    const searchSessions = vi.spyOn(agentService, "searchSessions");
    const listSessionCategories = vi.spyOn(agentService, "listSessionCategories");
    const listMessages = vi.spyOn(agentService, "listMessages");
    const getActiveSession = vi.spyOn(agentService, "getActiveSession");

    renderSidebarWith(generateSessions(SESSION_LIST_VIRTUALIZE_THRESHOLD));

    for (const spy of [listSessions, listArchivedSessions, searchSessions, listSessionCategories, listMessages, getActiveSession]) {
      expect(spy).not.toHaveBeenCalled();
    }
  });
});
