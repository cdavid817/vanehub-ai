// @vitest-environment jsdom

import { Fragment } from "react";
import { fireEvent, screen, waitFor } from "@testing-library/react";
import { beforeAll, beforeEach, describe, expect, it, vi } from "vitest";
import { activateAppLanguage } from "../i18n";
import { renderWithAppProviders } from "../test/render";
import type {
  ExecutionSpanSummary,
  ExecutionTimeline,
} from "../types/execution-observability";

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

vi.mock("../components/measured-virtual-list", () => ({
  MeasuredVirtualList: <T,>({ items, renderItem, testId }: { items: readonly T[]; renderItem: (item: T, index: number) => unknown; testId?: string }) => (
    <div data-testid={testId}>
      {items.map((item, index) => <Fragment key={index}>{renderItem(item, index) as never}</Fragment>)}
    </div>
  ),
}));

import { ExecutionTimelineTab } from "./execution-timeline-tab";

function span(overrides: Partial<ExecutionSpanSummary> & { spanId: string }): ExecutionSpanSummary {
  return {
    parentSpanId: null,
    name: `span ${overrides.spanId}`,
    kind: "unknown",
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
    ...overrides,
  };
}

function timeline(spans: ExecutionSpanSummary[], events: ExecutionTimeline["events"] = []): ExecutionTimeline {
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
      sessionId: "session-1",
      operationId: null,
      agentId: "claude-code",
    },
    spans,
    events,
  };
}

function mount(data: ExecutionTimeline) {
  mockService.listRuns.mockResolvedValue({ items: [data.run], nextPageToken: null });
  mockService.getTimeline.mockResolvedValue(data);
  return renderWithAppProviders(
    <ExecutionTimelineTab sessionId="session-1" service={mockService} />,
  );
}

/**
 * The waterfall as somebody who cannot see it experiences it.
 *
 * A chart made of coloured rectangles says nothing to a screen reader, and the two facts hardest
 * to convey visually are the two that matter most: a span that is still running has no end, and a
 * span that could not be placed has no position. Both look like ordinary bars.
 */
describe("trace waterfall", () => {
  beforeAll(async () => {
    await activateAppLanguage("en");
  });

  beforeEach(() => {
    vi.clearAllMocks();
  });

  it("renders one row per span with an accessible name carrying its status and fidelity", async () => {
    mount(timeline([span({ spanId: "a", kind: "model" })]));

    const row = await screen.findByRole("listitem", { name: /span a/ });
    expect(row.getAttribute("aria-label")).toContain("Succeeded");
    expect(row.getAttribute("aria-label")).toContain("Native");
    expect(row.getAttribute("aria-label")).toContain("Model");
  });

  it("says a running span is still running rather than giving it a duration", async () => {
    mount(timeline([span({ spanId: "a", status: "running", endedAt: null, completedDurationMs: undefined })]));

    const row = await screen.findByRole("listitem", { name: /span a/ });
    // The visual cue is an open-ended bar, which conveys nothing without sight — and a duration
    // here would say the work finished.
    expect(row.getAttribute("aria-label")).toContain("Still running");
  });

  it("says a span could not be placed rather than drawing it at the start", async () => {
    mount(timeline([span({ spanId: "a", startOffsetMs: undefined })]));

    const row = await screen.findByRole("listitem", { name: /span a/ });
    expect(row.getAttribute("aria-label")).toContain("Not placeable");
    // And it says so on screen too, instead of a bar at the origin that would claim the work
    // happened when the run began.
    expect(screen.getByText("Not placeable")).toBeDefined();
  });

  it("names the critical path and a retry in the accessible label", async () => {
    mount(timeline([span({ spanId: "a", criticalPath: true, attempt: 3, delegated: true })]));

    const row = await screen.findByRole("listitem", { name: /span a/ });
    const label = row.getAttribute("aria-label") ?? "";
    expect(label).toContain("On the critical path");
    expect(label).toContain("Delegated");
    expect(label).toContain("Attempt 3");
  });

  it("indents a child span without hiding one whose parent is missing", async () => {
    mount(timeline([
      span({ spanId: "root" }),
      span({ spanId: "child", parentSpanId: "root" }),
      span({ spanId: "orphan", parentSpanId: "never-recorded" }),
    ]));

    await screen.findByRole("listitem", { name: /span root/ });
    // An orphan is real work. Dropping it because of something that happened to a different span
    // would remove it from the trace entirely.
    expect(screen.getByRole("listitem", { name: /span orphan/ })).toBeDefined();
  });
});

describe("trace keyboard selection", () => {
  beforeAll(async () => {
    await activateAppLanguage("en");
  });

  beforeEach(() => {
    vi.clearAllMocks();
  });

  it("moves the selection with the arrow keys", async () => {
    mount(timeline([span({ spanId: "a" }), span({ spanId: "b" }), span({ spanId: "c" })]));
    await screen.findByRole("listitem", { name: /span a/ });
    const list = screen.getByRole("application", { name: "Execution waterfall" });

    fireEvent.keyDown(list, { key: "ArrowDown" });

    // Roving selection rather than tab order: in a virtualized list the rows nobody scrolled to
    // are not in the DOM, so tabbing through them is not an option at all.
    await waitFor(() => {
      expect(screen.getByRole("listitem", { name: /span b/ }).getAttribute("aria-current")).toBe("true");
    });
  });

  it("clamps at the ends rather than wrapping", async () => {
    mount(timeline([span({ spanId: "a" }), span({ spanId: "b" })]));
    await screen.findByRole("listitem", { name: /span a/ });
    const list = screen.getByRole("application", { name: "Execution waterfall" });

    fireEvent.keyDown(list, { key: "ArrowUp" });

    // Wrapping from the first row to the last is disorienting where vertical position carries
    // meaning: the reader would jump from the start of the run to its end with no sign they had.
    await waitFor(() => {
      expect(screen.getByRole("listitem", { name: /span a/ }).getAttribute("aria-current")).toBe("true");
    });
  });

  it("opens the detail drawer on Enter and closes it on Escape", async () => {
    mount(timeline([span({ spanId: "a" })]));
    await screen.findByRole("listitem", { name: /span a/ });
    const list = screen.getByRole("application", { name: "Execution waterfall" });

    fireEvent.keyDown(list, { key: "Enter" });
    await screen.findByRole("region", { name: "Span detail" });

    fireEvent.keyDown(list, { key: "Escape" });

    // Closing does not move the selection, so the reader returns to the row they were on rather
    // than to wherever the list started.
    await waitFor(() => {
      expect(screen.queryByRole("region", { name: "Span detail" })).toBeNull();
    });
    expect(screen.getByRole("listitem", { name: /span a/ }).getAttribute("aria-current")).toBe("true");
  });

  it("does not open the drawer merely by moving through rows", async () => {
    mount(timeline([span({ spanId: "a" }), span({ spanId: "b" })]));
    await screen.findByRole("listitem", { name: /span a/ });
    const list = screen.getByRole("application", { name: "Execution waterfall" });

    fireEvent.keyDown(list, { key: "ArrowDown" });

    // A drawer that opened on every arrow key would make the list unusable to navigate.
    expect(screen.queryByRole("region", { name: "Span detail" })).toBeNull();
  });
});

describe("trace detail drawer", () => {
  beforeAll(async () => {
    await activateAppLanguage("en");
  });

  beforeEach(() => {
    vi.clearAllMocks();
  });

  async function open(data: ExecutionTimeline, spanName = /span a/) {
    mount(data);
    const row = await screen.findByRole("listitem", { name: spanName });
    fireEvent.click(row);
    return screen.findByRole("region", { name: "Span detail" });
  }

  it("shows the overview, and says a running span has no duration", async () => {
    const drawer = await open(timeline([
      span({ spanId: "a", status: "running", endedAt: null, completedDurationMs: undefined }),
    ]));

    expect(drawer.textContent).toContain("Overview");
    expect(drawer.textContent).toContain("Still running");
  });

  it("separates usage attributes from the rest", async () => {
    const drawer = await open(timeline([
      span({
        spanId: "a",
        attributes: { "gen_ai.usage.input_tokens": 120, "gen_ai.tool.name": "search" },
      }),
    ]));

    // Usage is the one group a reader scans for a number rather than reading, so it gets its own
    // heading instead of sitting among twenty other attributes.
    expect(drawer.textContent).toContain("Usage");
    expect(drawer.textContent).toContain("gen_ai.usage.input_tokens");
    expect(drawer.textContent).toContain("Attributes");
    expect(drawer.textContent).toContain("gen_ai.tool.name");
  });

  it("shows an error as a classification code and never a message", async () => {
    const drawer = await open(timeline([
      span({ spanId: "a", status: "failed", errorClassification: "missing_terminal_boundary" }),
    ]));

    expect(drawer.textContent).toContain("Error");
    // A stable code, never prose: a message would be the one place in this panel where
    // unredacted producer text could appear.
    expect(drawer.textContent).toContain("missing_terminal_boundary");
  });

  it("lists only the events belonging to the selected span", async () => {
    const drawer = await open(timeline(
      [span({ spanId: "a" }), span({ spanId: "b" })],
      [
        { sequence: 1, spanId: "a", name: "process.spawned", timestamp: "2026-08-25T10:00:00.500Z", attributes: {} },
        { sequence: 2, spanId: "b", name: "tool.invoked", timestamp: "2026-08-25T10:00:00.700Z", attributes: {} },
      ],
    ));

    expect(drawer.textContent).toContain("process.spawned");
    expect(drawer.textContent).not.toContain("tool.invoked");
  });

  it("groups links under the section their relationship names", async () => {
    const drawer = await open(timeline([
      span({
        spanId: "a",
        links: [
          { runId: "run-1", traceId: "trace-1", spanId: "log-1", relationship: "log" },
          { runId: "run-1", traceId: "trace-1", spanId: "cmd-1", relationship: "command" },
          { runId: "run-1", traceId: "trace-1", spanId: "odd-1", relationship: "something-new" },
        ],
      }),
    ]));

    expect(drawer.textContent).toContain("Logs");
    expect(drawer.textContent).toContain("Commands");
    // An unrecognised relationship goes to "Other links" rather than being guessed into a section:
    // a log listed under Files is worse than one under a heading that admits it does not know.
    expect(drawer.textContent).toContain("Other links");
    expect(drawer.textContent).toContain("something-new");
  });

  it("says nothing is linked rather than implying nothing exists", async () => {
    const drawer = await open(timeline([span({ spanId: "a" })]));

    // This section lists what the span points at. A span that pointed at nothing is not evidence
    // that nothing happened.
    expect(drawer.textContent).toContain("Nothing linked.");
  });

  it("closes from its own control", async () => {
    await open(timeline([span({ spanId: "a" })]));

    fireEvent.click(screen.getByRole("button", { name: "Close span detail" }));

    await waitFor(() => {
      expect(screen.queryByRole("region", { name: "Span detail" })).toBeNull();
    });
  });
});

describe("trace toolbar", () => {
  beforeAll(async () => {
    await activateAppLanguage("en");
  });

  beforeEach(() => {
    vi.clearAllMocks();
  });

  it("names every colour the bars use", async () => {
    mount(timeline([span({ spanId: "a" })]));
    await screen.findByRole("listitem", { name: /span a/ });

    const legend = screen.getByRole("group", { name: "Legend" });
    // A colour with no key is a decoration. Each entry names something the reader can act on.
    expect(legend.textContent).toContain("Failed");
    expect(legend.textContent).toContain("Critical path");
    expect(legend.textContent).toContain("Delegated");
    expect(legend.textContent).toContain("Observation gap");
  });

  it("zooms in and out within its bounds", async () => {
    mount(timeline([span({ spanId: "a" })]));
    await screen.findByRole("listitem", { name: /span a/ });

    // At the minimum the whole run fits, so there is nothing to zoom out to.
    expect(screen.getByRole("button", { name: "Zoom out" }).hasAttribute("disabled")).toBe(true);

    fireEvent.click(screen.getByRole("button", { name: "Zoom in" }));

    await waitFor(() => {
      expect(screen.getByRole("button", { name: "Zoom out" }).hasAttribute("disabled")).toBe(false);
    });
  });
});
