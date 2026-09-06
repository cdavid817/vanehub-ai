// @vitest-environment jsdom

import { Fragment } from "react";
import { fireEvent, screen, waitFor } from "@testing-library/react";
import { beforeEach, describe, expect, it, vi } from "vitest";
import { activateAppLanguage } from "../i18n";
import { renderWithAppProviders } from "../test/render";
import type {
  ExecutionRunSummary,
  ExecutionSpanSummary,
  ExecutionTimeline,
} from "../types/execution-observability";
import type { TraceTransitionNotice } from "../types/trace-transition";

const { mockService } = vi.hoisted(() => ({
  mockService: {
    listRuns: vi.fn(),
    getRun: vi.fn(),
    getTimeline: vi.fn(),
    getSettings: vi.fn(),
    updateSettings: vi.fn(),
    getObservationCapabilities: vi.fn(),
  },
}));

// jsdom gives every element a zero height, so the real virtualizer renders no rows at all and
// every query below would fail for a reason unrelated to what is being tested.
vi.mock("../components/measured-virtual-list", () => ({
  MeasuredVirtualList: <T,>({
    items,
    renderItem,
    testId,
  }: {
    items: readonly T[];
    renderItem: (item: T, index: number) => unknown;
    testId?: string;
  }) => (
    <div data-testid={testId}>
      {items.map((item, index) => (
        <Fragment key={index}>{renderItem(item, index) as never}</Fragment>
      ))}
    </div>
  ),
}));

import { ExecutionTimelineTab } from "./execution-timeline-tab";

/** The run list shows the agent, not the id, so that is what identifies a row on screen. */
function run(index: number): ExecutionRunSummary {
  return {
    runId: `run-${index}`,
    traceId: `trace-${index}`,
    rootSpanId: `span-${index}`,
    source: "desktop",
    sourceId: null,
    status: "succeeded",
    startedAt: `2026-08-25T10:0${index}:00.000Z`,
    endedAt: `2026-08-25T10:0${index}:05.000Z`,
    durationMs: 5000,
    sessionId: "session-1",
    operationId: null,
    agentId: `agent-${index}`,
  };
}

function span(spanId: string): ExecutionSpanSummary {
  return {
    spanId,
    parentSpanId: null,
    name: `span ${spanId}`,
    kind: "process",
    status: "succeeded",
    fidelity: "native",
    startedAt: "2026-08-25T10:00:00.000Z",
    endedAt: "2026-08-25T10:00:01.000Z",
    durationMs: 1000,
    errorClassification: null,
    attributes: {},
    depth: 0,
    startOffsetMs: 0,
    completedDurationMs: 1000,
    delegated: false,
    criticalPath: false,
    links: [],
  };
}

function timeline(runId: string): ExecutionTimeline {
  return {
    run: run(Number(runId.replace("run-", ""))),
    spans: [span(`${runId}-a`), span(`${runId}-b`)],
    events: [],
    eventCoverage: { truncated: false, nextPageToken: null },
  };
}

/**
 * A live refresh must not behave like a navigation.
 *
 * One modelling error wearing several faces: a refresh counter in the query key makes every
 * refresh a *different query*, which starts empty. Loaded pages vanish, the correction effect sees
 * an empty list and clears the selection, and the viewport owning zoom, filters and the open
 * drawer unmounts along with its data. None of it looks like a bug in the refresh path, which is
 * how it survived a suite that already covered the refresh path.
 */
describe("trace live refresh preserves reader state", () => {
  let listeners: ((notice: TraceTransitionNotice) => void)[] = [];

  const subscribe = (listener: (notice: TraceTransitionNotice) => void) => {
    listeners.push(listener);
    return () => {
      listeners = listeners.filter((item) => item !== listener);
    };
  };

  function emit(overrides: Partial<TraceTransitionNotice> = {}) {
    for (const listener of [...listeners]) {
      listener({
        kind: "span-finished",
        runId: "run-1",
        traceId: "trace-1",
        spanId: "span-x",
        status: "succeeded",
        affectsRunList: false,
        ...overrides,
      });
    }
  }

  beforeEach(async () => {
    await activateAppLanguage("en");
    listeners = [];
    vi.clearAllMocks();
    mockService.listRuns.mockImplementation(async ({ pageToken }: { pageToken?: string | null }) =>
      pageToken === "page-2"
        ? { items: [run(3), run(4)], nextPageToken: null }
        : { items: [run(1), run(2)], nextPageToken: "page-2" },
    );
    mockService.getTimeline.mockImplementation(async (runId: string) => timeline(runId));
  });

  function mount() {
    return renderWithAppProviders(
      <ExecutionTimelineTab sessionId="session-1" service={mockService} subscribe={subscribe} />,
    );
  }

  it("keeps loaded run-list pages across a refresh", async () => {
    mount();
    await screen.findByText("agent-1");

    fireEvent.click(await screen.findByRole("button", { name: "Load older runs" }));
    await screen.findByText("agent-3");

    // A run transition is what re-reads the list, so that is the notice that must not discard it.
    emit({ kind: "run-started", runId: "run-9", affectsRunList: true, spanId: undefined });
    await new Promise((resolve) => setTimeout(resolve, 700));

    // The second page was fetched under the same data identity, so a refresh has no reason to
    // discard it. Under the counter key it belonged to a query nobody would ask for again.
    await waitFor(() => expect(screen.getByText("agent-3")).not.toBeNull(), { timeout: 2000 });
  });

  it("does not announce a newer run when the reader's own run merely finished", async () => {
    mount();
    await screen.findByText("agent-1");

    // A finish sets the same run-list flag a start does, and the notice carries no session id.
    // Deriving the banner from that flag announced a new run whenever anything ended anywhere.
    emit({ kind: "run-finished", runId: "run-1", affectsRunList: true, spanId: undefined });
    await new Promise((resolve) => setTimeout(resolve, 700));

    expect(screen.queryByRole("button", { name: /newer run/i })).toBeNull();
  });

  it("announces a newer run only when one actually reaches the top of this session's list", async () => {
    mount();
    await screen.findByText("agent-1");

    // The list itself changes: a genuinely new run for this session arrives at the top.
    mockService.listRuns.mockImplementation(async ({ pageToken }: { pageToken?: string | null }) =>
      pageToken === "page-2"
        ? { items: [run(3), run(4)], nextPageToken: null }
        : { items: [run(9), run(1), run(2)], nextPageToken: "page-2" },
    );
    emit({ kind: "run-started", runId: "run-9", affectsRunList: true, spanId: undefined });

    await screen.findByRole("button", { name: /newer run/i }, { timeout: 3000 });
  });

  it("keeps the reader's selected run across a refresh", async () => {
    mount();
    await screen.findByText("agent-2");

    fireEvent.click(screen.getByText("agent-2").closest("button") as HTMLElement);
    await waitFor(() => {
      const calls = mockService.getTimeline.mock.calls;
      expect(calls.at(-1)?.[0]).toBe("run-2");
    });

    // A new run starts while the reader is looking at an older one -- the case the run list has to
    // survive, because this is the notice that re-reads it.
    emit({ kind: "run-started", runId: "run-9", affectsRunList: true, spanId: undefined });

    // Selection belongs to the reader. Silently reselecting the newest run is a navigation they
    // did not ask for, performed while they are reading something else.
    await new Promise((resolve) => setTimeout(resolve, 700));
    const calls = mockService.getTimeline.mock.calls;
    expect(calls.at(-1)?.[0]).toBe("run-2");
  });
});
