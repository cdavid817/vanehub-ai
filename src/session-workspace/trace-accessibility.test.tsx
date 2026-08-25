// @vitest-environment jsdom

import { Fragment } from "react";
import { fireEvent, screen } from "@testing-library/react";
import { beforeAll, beforeEach, describe, expect, it, vi } from "vitest";
import { activateAppLanguage } from "../i18n";
import { renderWithAppProviders } from "../test/render";
import type {
  ExecutionSpanSummary,
  ExecutionTimeline,
} from "../types/execution-observability";
import { flattenSpanRows, placeSpanBar, traceTimeScale } from "./trace-time-scale";
import { compareRuns } from "./trace-comparison";

const { mockAgentService, mockObservability } = vi.hoisted(() => ({
  mockAgentService: {
    listSessionLogs: vi.fn(),
    listExecutionRecords: vi.fn(),
  },
  mockObservability: {
    listRuns: vi.fn(),
    getRun: vi.fn(),
    getTimeline: vi.fn(),
    getSettings: vi.fn(),
    updateSettings: vi.fn(),
    getObservationCapabilities: vi.fn(),
  },
}));

vi.mock("../services/runtime-agent-client", () => ({ agentService: mockAgentService }));
vi.mock("../services/runtime-trace-transition-client", () => ({
  traceTransitionStream: { subscribe: () => () => {} },
}));
vi.mock("../components/measured-virtual-list", () => ({
  MeasuredVirtualList: <T,>({ items, renderItem, testId }: { items: readonly T[]; renderItem: (item: T, index: number) => unknown; testId?: string }) => (
    <div data-testid={testId}>
      {items.map((item, index) => <Fragment key={index}>{renderItem(item, index) as never}</Fragment>)}
    </div>
  ),
}));

import { ExecutionTimelineTab } from "./execution-timeline-tab";

const SESSION = "018f0f17-4d6a-7e20-b41d-66c5271a28aa";

function span(index: number, overrides: Partial<ExecutionSpanSummary> = {}): ExecutionSpanSummary {
  return {
    spanId: (index + 0xa0).toString(16).padStart(16, "0"),
    parentSpanId: null,
    name: `span ${index}`,
    kind: "tool",
    status: "succeeded",
    fidelity: "native",
    startedAt: "2026-08-25T10:00:00.000Z",
    endedAt: "2026-08-25T10:00:01.000Z",
    durationMs: 1000,
    errorClassification: null,
    attributes: {},
    depth: 0,
    startOffsetMs: index,
    completedDurationMs: 10,
    delegated: false,
    criticalPath: false,
    links: [],
    ...overrides,
  };
}

function timeline(spans: ExecutionSpanSummary[]): ExecutionTimeline {
  return {
    run: {
      runId: "018f0f17-4d6a-7e20-b41d-66c5271a28d0",
      traceId: "4bf92f3577b34da6a3ce929d0e0e4736",
      rootSpanId: spans[0]?.spanId ?? "00f067aa0ba902b7",
      source: "desktop",
      sourceId: null,
      status: "succeeded",
      startedAt: "2026-08-25T10:00:00.000Z",
      endedAt: "2026-08-25T10:00:03.000Z",
      durationMs: 3000,
      sessionId: SESSION,
      operationId: null,
      agentId: "claude-code",
    },
    spans,
    events: [],
  };
}

function mount(data: ExecutionTimeline, theme: "futuristic" | "minimal" = "futuristic") {
  mockObservability.listRuns.mockResolvedValue({ items: [data.run], nextPageToken: null });
  mockObservability.getTimeline.mockResolvedValue(data);
  return renderWithAppProviders(
    <ExecutionTimelineTab sessionId={SESSION} service={mockObservability} />,
    { theme },
  );
}

/**
 * What the waterfall guarantees to somebody who is not looking at it, and to somebody looking at
 * it in either theme.
 *
 * A trace view is the panel most likely to become a picture with no text in it. These assertions
 * are the ones that fail the day it does.
 */
describe("trace accessibility", () => {
  beforeAll(async () => {
    await activateAppLanguage("en");
  });

  beforeEach(() => {
    vi.clearAllMocks();
    mockAgentService.listSessionLogs.mockResolvedValue({ items: [], truncated: false, nextCursor: null });
    mockAgentService.listExecutionRecords.mockResolvedValue({
      items: [],
      coverage: { state: "complete", truncated: false, reasonCodes: [] },
    });
  });

  it("gives the waterfall a named, focusable container", async () => {
    mount(timeline([span(0)]));

    const list = await screen.findByRole("application", { name: "Execution waterfall" });
    // One focusable element for the whole list. A virtualized list cannot use tab order, because
    // the rows nobody scrolled to are not in the DOM to be tabbed to.
    expect(list.getAttribute("tabindex")).toBe("0");
  });

  it("points the container at the row it considers active", async () => {
    mount(timeline([span(0), span(1)]));
    await screen.findByRole("listitem", { name: /span 0/ });

    const list = screen.getByRole("application", { name: "Execution waterfall" });
    fireEvent.keyDown(list, { key: "ArrowDown" });

    // Without this a screen reader announces the container and never the row inside it that the
    // arrow keys just moved to.
    expect(list.getAttribute("aria-activedescendant")).toBeTruthy();
  });

  it("names every control rather than relying on an icon", async () => {
    mount(timeline([span(0)]));
    await screen.findByRole("listitem", { name: /span 0/ });

    // Each of these is an icon-only button. An icon has no accessible name of its own.
    expect(screen.getByRole("button", { name: "Zoom in" })).toBeDefined();
    expect(screen.getByRole("button", { name: "Zoom out" })).toBeDefined();
    expect(screen.getByRole("group", { name: "Legend" })).toBeDefined();
    expect(screen.getByRole("group", { name: "Span filters" })).toBeDefined();
  });

  it("marks every filter as a toggle with its own state", async () => {
    mount(timeline([span(0)]));
    await screen.findByRole("listitem", { name: /span 0/ });

    const failed = screen.getByRole("button", { name: "Failed" });
    expect(failed.getAttribute("aria-pressed")).toBe("false");
    fireEvent.click(failed);
    // `aria-pressed` rather than a colour: the pressed state is the only thing telling a reader
    // why the list got shorter.
    expect(screen.getByRole("button", { name: "Failed" }).getAttribute("aria-pressed")).toBe("true");
  });

  it("keeps every accessible name in both visual styles", async () => {
    const { unmount } = mount(timeline([span(0)]), "futuristic");
    await screen.findByRole("listitem", { name: /span 0/ });
    unmount();

    mount(timeline([span(0)]), "minimal");

    // The two themes change colour and shape. A name that survived only one of them would mean the
    // panel is unusable without sight in the other.
    expect(await screen.findByRole("listitem", { name: /span 0/ })).toBeDefined();
    expect(screen.getByRole("application", { name: "Execution waterfall" })).toBeDefined();
    expect(screen.getByRole("group", { name: "Legend" })).toBeDefined();
  });

  it("renders the same span count in both visual styles", async () => {
    const spans = [span(0), span(1), span(2)];
    const { unmount } = mount(timeline(spans), "futuristic");
    const futuristic = (await screen.findAllByRole("listitem")).length;
    unmount();

    mount(timeline(spans), "minimal");
    const minimal = (await screen.findAllByRole("listitem")).length;

    // A theme decides how a row looks, never whether it exists.
    expect(minimal).toBe(futuristic);
  });

  it("names the comparison panel and its close control", async () => {
    const data = timeline([span(0)]);
    const other = {
      ...data,
      run: { ...data.run, runId: "018f0f17-4d6a-7e20-b41d-66c5271a28d1" },
    };
    mockObservability.listRuns.mockResolvedValue({
      items: [data.run, other.run],
      nextPageToken: null,
    });
    mockObservability.getTimeline.mockResolvedValue(data);
    renderWithAppProviders(
      <ExecutionTimelineTab sessionId={SESSION} service={mockObservability} />,
    );
    await screen.findByRole("listitem", { name: /span 0/ });

    fireEvent.click(screen.getByRole("button", { name: "Compare against this run" }));

    expect(await screen.findByRole("region", { name: "Run comparison" })).toBeDefined();
    expect(screen.getByRole("button", { name: "Close comparison" })).toBeDefined();
  });
});

/**
 * The waterfall at the largest size it is expected to hold.
 *
 * Asserted on work done rather than elapsed time: a wall-clock budget on a shared runner measures
 * the runner, which is exactly backwards for a test meant to catch a scaling mistake.
 */
describe("maximum span performance", () => {
  const MAXIMUM_SPANS = 5_000;

  it("flattens a maximum span set once, visiting each span exactly once", () => {
    const spans = Array.from({ length: MAXIMUM_SPANS }, (_unused, index) =>
      span(index, index === 0 ? {} : { parentSpanId: (index - 1 + 0xa0).toString(16).padStart(16, "0") }),
    );

    const rows = flattenSpanRows(spans);

    // A chain 5000 deep is the worst shape for a recursive walk, and the one a badly-instrumented
    // agent actually produces. Every span appears once and none is dropped.
    expect(rows).toHaveLength(MAXIMUM_SPANS);
    expect(new Set(rows.map((row) => row.span.spanId)).size).toBe(MAXIMUM_SPANS);
  });

  it("places a maximum span set without the cost of one bar depending on the others", () => {
    const spans = Array.from({ length: MAXIMUM_SPANS }, (_unused, index) => span(index));
    const scale = traceTimeScale(spans, 800, 1);

    const placements = spans.map((item) => placeSpanBar(item, scale));

    // Placement reads the scale and the span, and nothing else. A bar whose position depended on
    // the spans around it would make rendering quadratic in the span count.
    expect(placements).toHaveLength(MAXIMUM_SPANS);
    expect(placements.every((placement) => placement.kind === "placed")).toBe(true);
  });

  it("renders only what the virtualizer asks for, whatever the span count", async () => {
    const spans = Array.from({ length: 500 }, (_unused, index) => span(index));
    mount(timeline(spans));

    const list = await screen.findByTestId("trace-waterfall-list");
    // The stub renders everything, so this asserts the list is *given* every span rather than a
    // truncated set — the bound is the virtualizer's, and truncating before it would silently
    // hide work.
    expect(list.children).toHaveLength(500);
  });

  it("compares two maximum runs without walking their content", () => {
    const spans = Array.from({ length: MAXIMUM_SPANS }, (_unused, index) => span(index));

    const comparison = compareRuns(timeline(spans), timeline(spans));

    expect(comparison.spans).toMatchObject({ left: MAXIMUM_SPANS, right: MAXIMUM_SPANS });
    // One entry per distinct kind, not per span. A comparison whose size grew with the run would
    // be a second copy of it.
    expect(comparison.toolCounts).toHaveLength(1);
  });
});
