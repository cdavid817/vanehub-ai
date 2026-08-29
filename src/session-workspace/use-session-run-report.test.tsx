// @vitest-environment jsdom

import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { render, screen, waitFor } from "@testing-library/react";
import type { ReactElement, ReactNode } from "react";
import { describe, expect, it, vi } from "vitest";
import { EvidenceUnavailableError } from "../services/native-evidence-transport";
import type { SessionWorkspaceEvidenceService } from "../services/session-workspace-evidence-service";
import type { EvidenceSessionId, SessionRunReport } from "../types/session-workspace-evidence";
import { emptySessionRunReport } from "./report-test-fixtures";
import {
  useSessionRunReport,
  WHOLE_SESSION_REPORT_SCOPE,
  type ReportScopeSelection,
} from "./use-session-run-report";

const SESSION = "session-1" as EvidenceSessionId;

function wrap(ui: ReactElement): ReactElement {
  const queryClient = new QueryClient({
    defaultOptions: { mutations: { retry: false }, queries: { retry: false } },
  });
  return <QueryClientProvider client={queryClient}>{ui}</QueryClientProvider>;
}

function Probe({
  isVisible = true,
  scope = WHOLE_SESSION_REPORT_SCOPE,
  service,
  sessionId = SESSION,
}: {
  isVisible?: boolean;
  scope?: ReportScopeSelection;
  service: SessionWorkspaceEvidenceService;
  sessionId?: EvidenceSessionId | null;
}) {
  const result = useSessionRunReport({ isVisible, scope, service, sessionId });
  return (
    <output data-testid="probe">
      {`${result.state}:${result.isRefreshing}:${result.reasonCode ?? "-"}:${result.report?.overview.runCount ?? "-"}`}
    </output>
  );
}

function serviceReturning(...reports: (SessionRunReport | Error)[]) {
  const queue = [...reports];
  return {
    getSessionRunReport: vi.fn(async () => {
      const next = queue.length > 1 ? queue.shift() : queue[0];
      if (next instanceof Error) throw next;
      return next as SessionRunReport;
    }),
  } as unknown as SessionWorkspaceEvidenceService & {
    getSessionRunReport: ReturnType<typeof vi.fn>;
  };
}

function withRuns(count: number): SessionRunReport {
  const report = emptySessionRunReport(SESSION);
  return { ...report, overview: { ...report.overview, runCount: count } };
}

async function probeText(): Promise<string> {
  return (await screen.findByTestId("probe")).textContent ?? "";
}

describe("useSessionRunReport", () => {
  it("reads nothing without a session", async () => {
    const service = serviceReturning(withRuns(1));
    render(wrap(<Probe service={service} sessionId={null} />));

    // Null is a state, not an error: the workspace mounts panels before a session is chosen.
    expect(await probeText()).toContain("loading");
    expect(service.getSessionRunReport).not.toHaveBeenCalled();
  });

  it("reads nothing while the panel is hidden", async () => {
    const service = serviceReturning(withRuns(1));
    render(wrap(<Probe isVisible={false} service={service} />));

    await waitFor(() => expect(service.getSessionRunReport).not.toHaveBeenCalled());
  });

  it("refetches when the scope changes and keeps the previous report meanwhile", async () => {
    const service = serviceReturning(withRuns(3), withRuns(1));
    const { rerender } = render(wrap(<Probe service={service} />));
    await waitFor(async () => expect(await probeText()).toBe("ready:false:-:3"));

    rerender(
      wrap(<Probe scope={{ ...WHOLE_SESSION_REPORT_SCOPE, groupBy: "agent" }} service={service} />),
    );

    // The previous answer is still on screen while the narrower one is in flight, and the hook says
    // which of the two states it is in rather than leaving a reader to guess from a blank page.
    await waitFor(async () => expect(await probeText()).toBe("ready:true:-:3"));
    await waitFor(async () => expect(await probeText()).toBe("ready:false:-:1"));
  });

  it("surfaces a typed refusal code", async () => {
    const service = serviceReturning(new EvidenceUnavailableError("report_too_many_runs"));
    render(wrap(<Probe service={service} />));

    await waitFor(async () =>
      expect(await probeText()).toBe("unavailable:false:report_too_many_runs:-"),
    );
  });

  it("collapses an untyped failure to a generic code rather than surfacing its text", async () => {
    const service = serviceReturning(new Error("connection reset by peer at 10.0.0.4"));
    render(wrap(<Probe service={service} />));

    // Untranslated, and possibly naming internals. A panel showing it would show the reader
    // something no locale file has a string for.
    await waitFor(async () =>
      expect(await probeText()).toBe("unavailable:false:evidence_unavailable:-"),
    );
  });

  it("does not share a cache entry between two scopes", async () => {
    const service = serviceReturning(withRuns(3), withRuns(1));
    const queryClient = new QueryClient({
      defaultOptions: { mutations: { retry: false }, queries: { retry: false } },
    });
    const shared = (ui: ReactNode) => (
      <QueryClientProvider client={queryClient}>{ui}</QueryClientProvider>
    );

    const { rerender } = render(shared(<Probe service={service} />));
    await waitFor(async () => expect(await probeText()).toBe("ready:false:-:3"));

    rerender(
      shared(
        <Probe scope={{ ...WHOLE_SESSION_REPORT_SCOPE, from: "2026-08-25T00:00:00Z" }} service={service} />,
      ),
    );

    // A narrower scope that hit the wider one's entry would render the whole session's figures
    // under a filter, and nothing in the answer would say so.
    await waitFor(async () => expect(await probeText()).toBe("ready:false:-:1"));
    expect(service.getSessionRunReport).toHaveBeenCalledTimes(2);
  });
});
