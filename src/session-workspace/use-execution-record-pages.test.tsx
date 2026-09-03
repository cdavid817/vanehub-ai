// @vitest-environment jsdom

import { render, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { beforeEach, describe, expect, it, vi } from "vitest";
import {
  evidenceCursorSchema,
  evidenceRecordIdSchema,
  evidenceSessionIdSchema,
} from "../contracts/session-workspace-evidence-ids";
import type { SessionWorkspaceEvidenceService } from "../services/session-workspace-evidence-service";
import type {
  EvidenceStatus,
  ExecutionRecord,
  QueryCoverage,
  WorkspaceEvidenceScope,
} from "../types/session-workspace-evidence";
import { mergeRecordPage, useExecutionRecordPages } from "./use-execution-record-pages";

const sessionId = evidenceSessionIdSchema.parse("session-a");
const scope: WorkspaceEvidenceScope = { sessionId };
const complete: QueryCoverage = { state: "complete", reasonCodes: [], truncated: false };

function record(id: string, status: EvidenceStatus = "running"): ExecutionRecord {
  return {
    id: evidenceRecordIdSchema.parse(id),
    kind: "tool",
    sessionId,
    status,
    fidelity: "native",
    coverage: complete,
    toolName: id,
    source: "native",
  };
}

function serviceDouble(list: ReturnType<typeof vi.fn>): SessionWorkspaceEvidenceService {
  return {
    getWorkspaceEvidenceSummary: vi.fn(),
    listExecutionRecords: list,
    getExecutionRecord: vi.fn(),
    subscribeExecutionEvidence: vi.fn(),
    getSessionRunReport: vi.fn(),
  } as unknown as SessionWorkspaceEvidenceService;
}

function Probe({
  refreshToken = 0,
  service,
}: {
  refreshToken?: number;
  service: SessionWorkspaceEvidenceService;
}) {
  const pages = useExecutionRecordPages({ filters: {}, refreshToken, scope, service });
  return (
    <div>
      <span data-testid="ids">{pages.records.map((entry) => entry.id).join(",")}</span>
      <span data-testid="statuses">{pages.records.map((entry) => entry.status).join(",")}</span>
      <span data-testid="page-error">{pages.pageError ?? "-"}</span>
      <span data-testid="initial-error">{pages.initialError ?? "-"}</span>
      <span data-testid="has-more">{String(pages.hasMore)}</span>
      <button onClick={() => void pages.loadMore()} type="button">
        load more
      </button>
      <button onClick={() => void pages.retry()} type="button">
        retry
      </button>
    </div>
  );
}

describe("mergeRecordPage", () => {
  it("replaces a record that has since finished rather than adding a second", () => {
    // The id is the identity. A running row and its terminal update are one record; appending
    // would show the same work twice, once as still running.
    const merged = mergeRecordPage([record("a", "running")], [record("a", "succeeded")]);
    expect(merged).toHaveLength(1);
    expect(merged[0].status).toBe("succeeded");
  });

  it("keeps a boundary row from appearing on two pages", () => {
    const merged = mergeRecordPage([record("a"), record("b")], [record("b"), record("c")]);
    expect(merged.map((entry) => entry.id)).toEqual(["a", "b", "c"]);
  });

  it("appends in the order the server sent them", () => {
    const merged = mergeRecordPage([record("a")], [record("b"), record("c")]);
    expect(merged.map((entry) => entry.id)).toEqual(["a", "b", "c"]);
  });
});

describe("useExecutionRecordPages", () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  it("continues from the cursor the server issued", async () => {
    const user = userEvent.setup();
    const cursor = evidenceCursorSchema.parse("cursor-1");
    const list = vi
      .fn()
      .mockResolvedValueOnce({ items: [record("a")], nextCursor: cursor, coverage: complete })
      .mockResolvedValueOnce({ items: [record("b")], coverage: complete });
    render(<Probe service={serviceDouble(list)} />);

    await waitFor(() => expect(screen.getByTestId("ids").textContent).toBe("a"));
    expect(screen.getByTestId("has-more").textContent).toBe("true");
    await user.click(screen.getByRole("button", { name: "load more" }));

    await waitFor(() => expect(screen.getByTestId("ids").textContent).toBe("a,b"));
    expect(list.mock.calls[1][0].cursor).toBe(cursor);
    expect(screen.getByTestId("has-more").textContent).toBe("false");
  });

  it("keeps loaded rows when a continuation fails", async () => {
    const user = userEvent.setup();
    const cursor = evidenceCursorSchema.parse("cursor-1");
    const list = vi
      .fn()
      .mockResolvedValueOnce({ items: [record("a")], nextCursor: cursor, coverage: complete })
      .mockRejectedValueOnce(new Error("offline"));
    render(<Probe service={serviceDouble(list)} />);

    await waitFor(() => expect(screen.getByTestId("ids").textContent).toBe("a"));
    await user.click(screen.getByRole("button", { name: "load more" }));

    // A failed continuation says nothing about the page the reader is looking at.
    await waitFor(() => expect(screen.getByTestId("page-error").textContent).not.toBe("-"));
    expect(screen.getByTestId("ids").textContent).toBe("a");
    expect(screen.getByTestId("initial-error").textContent).toBe("-");
  });

  it("retries from the same boundary the failed attempt used", async () => {
    const user = userEvent.setup();
    const cursor = evidenceCursorSchema.parse("cursor-1");
    const list = vi
      .fn()
      .mockResolvedValueOnce({ items: [record("a")], nextCursor: cursor, coverage: complete })
      .mockRejectedValueOnce(new Error("offline"))
      .mockResolvedValueOnce({ items: [record("b")], coverage: complete });
    render(<Probe service={serviceDouble(list)} />);

    await waitFor(() => expect(screen.getByTestId("ids").textContent).toBe("a"));
    await user.click(screen.getByRole("button", { name: "load more" }));
    await waitFor(() => expect(screen.getByTestId("page-error").textContent).not.toBe("-"));
    await user.click(screen.getByRole("button", { name: "retry" }));

    // A retry that moved the boundary would skip whatever sat between the two, and nothing
    // downstream could tell that it had.
    await waitFor(() => expect(screen.getByTestId("ids").textContent).toBe("a,b"));
    expect(list.mock.calls[2][0].cursor).toBe(cursor);
  });

  it("blocks only when there was nothing to look at yet", async () => {
    const list = vi.fn().mockRejectedValue(new Error("offline"));
    render(<Probe service={serviceDouble(list)} />);

    await waitFor(() => expect(screen.getByTestId("initial-error").textContent).not.toBe("-"));
    expect(screen.getByTestId("ids").textContent).toBe("");
  });

  it("re-reads the newest page on a live revision without clearing what is loaded", async () => {
    const list = vi
      .fn()
      .mockResolvedValueOnce({ items: [record("a", "running")], coverage: complete })
      .mockResolvedValueOnce({ items: [record("a", "succeeded")], coverage: complete });
    const view = render(<Probe refreshToken={0} service={serviceDouble(list)} />);

    await waitFor(() => expect(screen.getByTestId("statuses").textContent).toBe("running"));
    view.rerender(<Probe refreshToken={1} service={serviceDouble(list)} />);

    // The row is replaced, not appended, and the reader keeps their place.
    await waitFor(() => expect(screen.getByTestId("statuses").textContent).toBe("succeeded"));
    expect(screen.getByTestId("ids").textContent).toBe("a");
  });

  it("asks for no more than the backend's own page bound", async () => {
    const list = vi.fn().mockResolvedValue({ items: [], coverage: complete });
    render(<Probe service={serviceDouble(list)} />);

    await waitFor(() => expect(list).toHaveBeenCalled());
    expect(list.mock.calls[0][0].limit).toBe(100);
  });
});
