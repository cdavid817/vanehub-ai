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
const RUN = "018f0f17-4d6a-7e20-b41d-66c5271a28d0";
const TRACE = "4bf92f3577b34da6a3ce929d0e0e4736";
const SPAN = "00f067aa0ba902b7";

function span(overrides: Partial<ExecutionSpanSummary> = {}): ExecutionSpanSummary {
  return {
    spanId: SPAN,
    parentSpanId: null,
    name: "vanehub.task.execute",
    kind: "tool",
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

function timeline(spans: ExecutionSpanSummary[] = [span()]): ExecutionTimeline {
  return {
    run: {
      runId: RUN,
      traceId: TRACE,
      rootSpanId: SPAN,
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

async function openDrawer(spans?: ExecutionSpanSummary[]) {
  const data = timeline(spans);
  mockObservability.listRuns.mockResolvedValue({ items: [data.run], nextPageToken: null });
  mockObservability.getTimeline.mockResolvedValue(data);
  renderWithAppProviders(
    <ExecutionTimelineTab sessionId={SESSION} service={mockObservability} />,
  );
  const row = await screen.findByRole("listitem", { name: /vanehub.task.execute/ });
  fireEvent.click(row);
  return screen.findByRole("region", { name: "Span detail" });
}

/**
 * A trace payload carries identifiers; the records they point at come from whoever owns them.
 *
 * Not tidiness. Log text and command output are exactly the material redaction exists for, and a
 * trace DTO is one of the places redaction has no second chance to run.
 */
describe("span linked evidence", () => {
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

  it("asks the log index for this span's own correlation", async () => {
    await openDrawer();

    // The index has filtered on `traceId` and `spanId` since it existed, so this is a lookup
    // rather than a match on anything derived.
    expect(mockAgentService.listSessionLogs).toHaveBeenCalledWith(
      expect.objectContaining({ sessionId: SESSION, traceId: TRACE, spanId: SPAN }),
    );
  });

  it("asks the evidence journal with the same scope", async () => {
    await openDrawer();

    expect(mockAgentService.listExecutionRecords).toHaveBeenCalledWith(
      expect.objectContaining({
        scope: expect.objectContaining({ sessionId: SESSION, runId: RUN, spanId: SPAN }),
      }),
    );
  });

  it("shows the linked log lines it was given", async () => {
    mockAgentService.listSessionLogs.mockResolvedValue({
      items: [{
        id: "log-1",
        timestamp: "2026-08-25T10:00:00.500Z",
        level: "error",
        category: "session.runtime",
        message: "the tool call failed",
        context: {},
      }],
      truncated: false,
      nextCursor: null,
    });

    await openDrawer();

    // Awaited: the queries settle after the drawer mounts, which is the ordinary case — the
    // section shows a loading line first and the rows when they arrive.
    expect(await screen.findByText("the tool call failed")).toBeDefined();
  });

  it("sorts commands and verifications into their own sections", async () => {
    mockAgentService.listExecutionRecords.mockResolvedValue({
      items: [
        {
          id: "record-1",
          kind: "command",
          sessionId: SESSION,
          status: "succeeded",
          fidelity: "native",
          startedAt: "2026-08-25T10:00:00.100Z",
          commandId: "cmd-1",
          runtimeKind: "local",
          redactedDisplay: "git status",
          outputAvailability: "available",
          outputTruncated: false,
        },
        {
          id: "record-2",
          kind: "verification",
          sessionId: SESSION,
          status: "failed",
          fidelity: "native",
          startedAt: "2026-08-25T10:00:00.200Z",
          verificationName: "unit-tests",
          outcome: "failed",
        },
      ],
      coverage: { state: "complete", truncated: false, reasonCodes: [] },
    });

    await openDrawer();

    // Commands are work; verifications are what a reader means by a finding. A tool call is not
    // folded into findings, because it reports no outcome.
    expect(await screen.findByText("git status")).toBeDefined();
    expect(await screen.findByText("unit-tests")).toBeDefined();
  });

  it("distinguishes a failed lookup from an empty result", async () => {
    mockAgentService.listSessionLogs.mockRejectedValue(new Error("index unavailable"));

    await openDrawer();

    // An empty section means the span linked to nothing; a failed one means nobody knows. Drawing
    // them the same way turns "we could not look" into "there is nothing there".
    // All three queried sections report it, because one failed lookup leaves all three unable to
    // say anything — and a section that stayed quiet would be the one claiming emptiness.
    expect(await screen.findAllByText(/Linked evidence could not be loaded/)).toHaveLength(3);
  });

  it("says nothing is linked when the queries answered with nothing", async () => {
    await openDrawer();

    expect(await screen.findAllByText("Nothing linked.")).not.toHaveLength(0);
  });

  it("shows a file link from the span itself, since nothing owns files yet", async () => {
    const drawer = await openDrawer([
      span({
        links: [{ runId: RUN, traceId: TRACE, spanId: "file-1", relationship: "file-change" }],
      }),
    ]);

    expect(drawer.textContent).toContain("Files");
    expect(drawer.textContent).toContain("file-change");
  });
});

describe("trace span filters", () => {
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

  async function mount(spans: ExecutionSpanSummary[]) {
    const data = timeline(spans);
    mockObservability.listRuns.mockResolvedValue({ items: [data.run], nextPageToken: null });
    mockObservability.getTimeline.mockResolvedValue(data);
    renderWithAppProviders(
      <ExecutionTimelineTab sessionId={SESSION} service={mockObservability} />,
    );
    return screen.findByRole("group", { name: "Span filters" });
  }

  it("narrows the waterfall and says how many it hid", async () => {
    await mount([
      span({ spanId: SPAN, name: "kept", status: "failed" }),
      span({ spanId: "b7ad6b7169203331", name: "hidden" }),
    ]);

    fireEvent.click(screen.getByRole("button", { name: "Failed" }));

    expect(await screen.findByText("1 hidden by filters")).toBeDefined();
    expect(screen.queryByRole("listitem", { name: /hidden/ })).toBeNull();
  });

  it("says every span is filtered out rather than that the run recorded none", async () => {
    await mount([span({ spanId: SPAN, name: "only" })]);

    fireEvent.click(screen.getByRole("button", { name: "Failed" }));

    // A run with no spans recorded nothing; a filtered-out one recorded plenty. Only the message
    // distinguishes them, and the difference is what a reader would otherwise get wrong.
    expect(await screen.findByText(/Every span in this run is hidden/)).toBeDefined();
  });

  it("restores everything when the filters are cleared", async () => {
    await mount([
      span({ spanId: SPAN, name: "kept", status: "failed" }),
      span({ spanId: "b7ad6b7169203331", name: "other" }),
    ]);
    fireEvent.click(screen.getByRole("button", { name: "Failed" }));
    await screen.findByText("1 hidden by filters");

    fireEvent.click(screen.getByRole("button", { name: "Clear filters" }));

    expect(await screen.findByRole("listitem", { name: /other/ })).toBeDefined();
  });
});
