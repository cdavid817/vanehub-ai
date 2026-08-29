// @vitest-environment jsdom

import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { render, waitFor } from "@testing-library/react";
import type { ReactNode } from "react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import {
  evidenceRecordIdSchema,
  evidenceSessionIdSchema,
} from "../contracts/session-workspace-evidence-ids";
import type { SessionWorkspaceEvidenceService } from "../services/session-workspace-evidence-service";
import type {
  ExecutionEvidenceNotice,
  WorkspaceEvidenceSummary,
} from "../types/session-workspace-evidence";
import { evidenceQueryKeys } from "./evidence-query-keys";
import {
  useWorkspaceEvidenceNotices,
  useWorkspaceEvidenceSummary,
} from "./use-workspace-evidence-summary";
import { EVIDENCE_NOTICE_WINDOW_MS } from "./workspace-evidence-notices";

const sessionId = evidenceSessionIdSchema.parse("session-a");
const otherSessionId = evidenceSessionIdSchema.parse("session-b");
const recordId = evidenceRecordIdSchema.parse("record-1");

const summary: WorkspaceEvidenceSummary = {
  sessionId,
  generatedAt: "2026-08-23T10:00:00.000Z",
  coverage: { state: "complete", reasonCodes: [], truncated: false },
  runState: { status: "running" },
  changes: { changedFiles: 1, unviewedFiles: 1 },
  executionRecords: { running: 1, failed: 0 },
  shells: { live: 0 },
  logs: { newErrors: 0 },
  traces: { running: 0, failed: 0 },
  verification: { passed: 0, failed: 0 },
  usage: { coverage: "complete" },
};

function serviceDouble(
  overrides: Partial<SessionWorkspaceEvidenceService> = {},
): SessionWorkspaceEvidenceService {
  return {
    getWorkspaceEvidenceSummary: vi.fn().mockResolvedValue(summary),
    listExecutionRecords: vi.fn(),
    getExecutionRecord: vi.fn(),
    subscribeExecutionEvidence: vi.fn().mockResolvedValue(() => undefined),
    getSessionRunReport: vi.fn(),
    ...overrides,
  } as SessionWorkspaceEvidenceService;
}

function mount(children: ReactNode, queryClient: QueryClient) {
  return render(<QueryClientProvider client={queryClient}>{children}</QueryClientProvider>);
}

function newClient() {
  return new QueryClient({
    defaultOptions: { mutations: { retry: false }, queries: { retry: false } },
  });
}

describe("useWorkspaceEvidenceSummary", () => {
  it("reads the summary once for the whole workspace", async () => {
    const service = serviceDouble();
    const queryClient = newClient();
    function Probe() {
      const { state, summary: value } = useWorkspaceEvidenceSummary(sessionId, service);
      return <output data-testid="state">{`${state}:${value?.changes.unviewedFiles ?? "-"}`}</output>;
    }
    const view = mount(
      <>
        <Probe />
        <Probe />
      </>,
      queryClient,
    );

    await waitFor(() =>
      expect(view.getAllByTestId("state").every((node) => node.textContent === "ready:1")).toBe(
        true,
      ),
    );
    // Two consumers, one request: the key is the session, so the second consumer reads the cache.
    expect(service.getWorkspaceEvidenceSummary).toHaveBeenCalledTimes(1);
  });

  it("reports a failed summary as unavailable rather than as an empty one", async () => {
    const service = serviceDouble({
      getWorkspaceEvidenceSummary: vi.fn().mockRejectedValue(new Error("not wired")),
    });
    const queryClient = newClient();
    function Probe() {
      const { state } = useWorkspaceEvidenceSummary(sessionId, service);
      return <output data-testid="state">{state}</output>;
    }
    const view = mount(<Probe />, queryClient);

    // An empty summary would put a confident zero on every badge.
    await waitFor(() => expect(view.getByTestId("state").textContent).toBe("unavailable"));
  });

  it("does not read anything before a session is selected", () => {
    const service = serviceDouble();
    function Probe() {
      useWorkspaceEvidenceSummary(null, service);
      return null;
    }
    mount(<Probe />, newClient());
    expect(service.getWorkspaceEvidenceSummary).not.toHaveBeenCalled();
  });
});

describe("useWorkspaceEvidenceNotices", () => {
  beforeEach(() => {
    vi.useFakeTimers({ shouldAdvanceTime: true });
  });

  afterEach(() => {
    vi.useRealTimers();
  });

  function subscribeDouble() {
    let emit: ((notice: ExecutionEvidenceNotice) => void) | null = null;
    const unsubscribe = vi.fn();
    const service = serviceDouble({
      subscribeExecutionEvidence: vi.fn(async (_input, listener) => {
        emit = listener;
        return unsubscribe;
      }),
    });
    return { emit: (notice: ExecutionEvidenceNotice) => emit?.(notice), service, unsubscribe };
  }

  function Probe({ service }: { service: SessionWorkspaceEvidenceService }) {
    useWorkspaceEvidenceNotices(sessionId, service);
    return <output data-testid="notices">rendered</output>;
  }

  it("subscribes once and folds a burst into one invalidation", async () => {
    const { emit, service } = subscribeDouble();
    const queryClient = newClient();
    const invalidate = vi.spyOn(queryClient, "invalidateQueries");
    mount(<Probe service={service} />, queryClient);

    await waitFor(() => expect(service.subscribeExecutionEvidence).toHaveBeenCalledTimes(1));
    for (let index = 1; index <= 20; index += 1) {
      emit({
        kind: "record-appended",
        sequence: index,
        sessionId,
        occurredAt: "2026-08-23T10:00:00.000Z",
      });
    }
    expect(invalidate).not.toHaveBeenCalled();

    await vi.advanceTimersByTimeAsync(EVIDENCE_NOTICE_WINDOW_MS);

    // Twenty appends are two invalidations — the records family and the summary — not forty.
    expect(invalidate).toHaveBeenCalledTimes(2);
  });

  it("ignores a notice belonging to another session", async () => {
    const { emit, service } = subscribeDouble();
    const queryClient = newClient();
    const invalidate = vi.spyOn(queryClient, "invalidateQueries");
    mount(<Probe service={service} />, queryClient);

    await waitFor(() => expect(service.subscribeExecutionEvidence).toHaveBeenCalledTimes(1));
    emit({
      kind: "summary-changed",
      sequence: 1,
      sessionId: otherSessionId,
      occurredAt: "2026-08-23T10:00:00.000Z",
    });
    await vi.advanceTimersByTimeAsync(EVIDENCE_NOTICE_WINDOW_MS * 2);

    expect(invalidate).not.toHaveBeenCalled();
  });

  it("invalidates only the summary key for a summary-changed notice", async () => {
    const { emit, service } = subscribeDouble();
    const queryClient = newClient();
    const invalidate = vi.spyOn(queryClient, "invalidateQueries");
    mount(<Probe service={service} />, queryClient);

    await waitFor(() => expect(service.subscribeExecutionEvidence).toHaveBeenCalledTimes(1));
    emit({
      kind: "summary-changed",
      sequence: 1,
      sessionId,
      occurredAt: "2026-08-23T10:00:00.000Z",
    });
    await vi.advanceTimersByTimeAsync(EVIDENCE_NOTICE_WINDOW_MS);

    expect(invalidate).toHaveBeenCalledTimes(1);
    expect(invalidate).toHaveBeenCalledWith({ queryKey: evidenceQueryKeys.summary(sessionId) });
  });

  it("widens to the session when a gap says rows were dropped", async () => {
    const { emit, service } = subscribeDouble();
    const queryClient = newClient();
    const invalidate = vi.spyOn(queryClient, "invalidateQueries");
    mount(<Probe service={service} />, queryClient);

    await waitFor(() => expect(service.subscribeExecutionEvidence).toHaveBeenCalledTimes(1));
    emit({
      kind: "coverage-gap",
      sequence: 9,
      sessionId,
      occurredAt: "2026-08-23T10:00:00.000Z",
      droppedCount: 4,
    });
    await vi.advanceTimersByTimeAsync(EVIDENCE_NOTICE_WINDOW_MS);

    // One predicate covering the whole session, because nothing narrower is honest after a gap.
    expect(invalidate).toHaveBeenCalledTimes(1);
    expect(invalidate.mock.calls[0][0]).toHaveProperty("predicate");
  });

  it("releases the subscription and drops a pending flush on unmount", async () => {
    const { emit, service, unsubscribe } = subscribeDouble();
    const queryClient = newClient();
    const invalidate = vi.spyOn(queryClient, "invalidateQueries");
    const view = mount(<Probe service={service} />, queryClient);

    await waitFor(() => expect(service.subscribeExecutionEvidence).toHaveBeenCalledTimes(1));
    emit({
      kind: "record-updated",
      sequence: 1,
      sessionId,
      occurredAt: "2026-08-23T10:00:00.000Z",
      recordId,
    });
    view.unmount();
    await vi.advanceTimersByTimeAsync(EVIDENCE_NOTICE_WINDOW_MS * 2);

    expect(unsubscribe).toHaveBeenCalled();
    expect(invalidate).not.toHaveBeenCalled();
  });
});
