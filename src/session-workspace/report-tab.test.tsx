// @vitest-environment jsdom

import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { render, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import type { ReactElement } from "react";
import { I18nextProvider } from "react-i18next";
import { beforeAll, beforeEach, describe, expect, it, vi } from "vitest";
import { activateAppLanguage, i18n } from "../i18n";
import { EvidenceUnavailableError } from "../services/native-evidence-transport";
import type { SessionWorkspaceEvidenceService } from "../services/session-workspace-evidence-service";
import type {
  EvidenceAgentId,
  EvidenceSessionId,
  SessionRunReport,
} from "../types/session-workspace-evidence";
import { ReportTab } from "./report-tab";
import { emptySessionRunReport } from "./report-test-fixtures";
import {
  useWorkspaceEvidenceScope,
  WorkspaceEvidenceScopeProvider,
} from "./workspace-evidence-scope";

const SESSION = "session-1" as EvidenceSessionId;

function service(report: SessionRunReport | Error) {
  return {
    getSessionRunReport: vi.fn(async () => {
      if (report instanceof Error) throw report;
      return report;
    }),
  } as unknown as SessionWorkspaceEvidenceService & {
    getSessionRunReport: ReturnType<typeof vi.fn>;
  };
}

function mount(ui: ReactElement) {
  const queryClient = new QueryClient({
    defaultOptions: { mutations: { retry: false }, queries: { retry: false } },
  });
  const wrap = (element: ReactElement) => (
    <I18nextProvider i18n={i18n}>
      <QueryClientProvider client={queryClient}>
        <WorkspaceEvidenceScopeProvider seatIds={[]} sessionId={SESSION}>
          {element}
        </WorkspaceEvidenceScopeProvider>
      </QueryClientProvider>
    </I18nextProvider>
  );
  const rendered = render(wrap(ui));
  return { rerenderPanel: (next: ReactElement) => rendered.rerender(wrap(next)) };
}

/** A report with something in every section, so a missing section is visible as a missing value. */
function populated(): SessionRunReport {
  const base = emptySessionRunReport(SESSION);
  return {
    ...base,
    overview: { runCount: 3, succeeded: 2, failed: 1, cancelled: 0, retries: 1, durationMs: 71_000 },
    usage: {
      ...base.usage,
      reportedInputTokens: 90_000,
      reportedOutputTokens: 22_000,
      estimatedCharacters: 40_000,
      responseCount: 9,
      internalPurposeResponseCount: 3,
    },
    latency: { p50Ms: 31, p95Ms: 12_400, slowestRecordDurationMs: 12_400 },
    agents: [
      { agentId: "claude-code" as EvidenceAgentId, runCount: 3, failedCount: 1, durationMs: 71_000 },
    ],
    tools: [{ toolName: "read_file", invocations: 12, failures: 1, durationMs: 1_200 }],
    commands: { total: 4, failed: 1, running: 0, durationMs: 3_000 },
    changes: { changedFiles: 7, unresolvedFindings: 2 },
    verification: { passed: 138, failed: 2, skipped: 1 },
    failures: { rows: [{ reasonCode: "command_failed_exit", count: 1 }] },
  };
}

describe("ReportTab", () => {
  beforeAll(async () => {
    await activateAppLanguage("en");
  });
  beforeEach(() => vi.clearAllMocks());

  it("renders every section from the backend report", async () => {
    mount(<ReportTab sessionId={SESSION} service={service(populated())} />);

    for (const section of [
      "Overview",
      "Usage",
      "Latency",
      "Agents",
      "Tools",
      "Commands",
      "Changes",
      "Tests",
      "Failures",
    ]) {
      await screen.findByRole("heading", { name: section });
    }
    expect(await screen.findByText("read_file")).toBeTruthy();
    expect(await screen.findByText("command_failed_exit")).toBeTruthy();
  });

  /**
   * 10.15: the report no longer moves when the conversation does.
   *
   * The panel it replaced summed whatever `ChatMessage[]` was mounted, so paging older messages in
   * changed every figure on the page and a trimmed history reported a smaller session. Nothing said
   * so — the numbers were equally confident either way. There is no message prop left to pass, so
   * what this asserts is the consequence: re-rendering the workspace while the mounted message count
   * changes issues no new read and leaves every figure identical.
   */
  it("does not change when the mounted message count changes", async () => {
    const evidence = service(populated());
    const { rerenderPanel } = mount(
      <MessageHarness count={3}>
        <ReportTab sessionId={SESSION} service={evidence} />
      </MessageHarness>,
    );
    await screen.findByRole("heading", { name: "Overview" });
    // Every figure on the page, not one of them: the old panel moved all of them together.
    const before = figures();
    await waitFor(() => expect(evidence.getSessionRunReport).toHaveBeenCalledTimes(1));

    rerenderPanel(
      <MessageHarness count={250}>
        <ReportTab sessionId={SESSION} service={evidence} />
      </MessageHarness>,
    );

    expect(figures()).toBe(before);
    // One read, not two: the report's inputs are the scope and the session, and neither moved.
    expect(evidence.getSessionRunReport).toHaveBeenCalledTimes(1);
  });

  it("asks the backend for the scope the controls describe", async () => {
    const evidence = service(populated());
    mount(<ReportTab sessionId={SESSION} service={evidence} />);
    await screen.findByRole("heading", { name: "Overview" });

    await userEvent.click(screen.getByRole("button", { name: "Seat" }));

    await waitFor(() =>
      expect(evidence.getSessionRunReport).toHaveBeenLastCalledWith(
        expect.objectContaining({ groupBy: "seat", sessionId: SESSION }),
      ),
    );
  });

  it("keeps the previous report on screen while a narrower one is fetched", async () => {
    let release: ((report: SessionRunReport) => void) | undefined;
    const evidence = {
      getSessionRunReport: vi
        .fn()
        .mockResolvedValueOnce(populated())
        .mockImplementationOnce(
          () =>
            new Promise<SessionRunReport>((resolve) => {
              release = resolve;
            }),
        ),
    } as unknown as SessionWorkspaceEvidenceService;

    mount(<ReportTab sessionId={SESSION} service={evidence} />);
    await screen.findByRole("heading", { name: "Overview" });
    const before = figures();

    await userEvent.click(screen.getByRole("button", { name: "Last hour" }));

    // The whole point of `keepPreviousData`: a reader mid-sentence does not lose the page because
    // they touched a control. The refresh says so beside the controls instead.
    expect(figures()).toBe(before);
    await screen.findByText("Refreshing");
    release?.(emptySessionRunReport(SESSION));
  });

  it("renders an em dash for a figure nothing measured", async () => {
    const report = emptySessionRunReport(SESSION);
    mount(<ReportTab sessionId={SESSION} service={service(report)} />);
    await screen.findByRole("heading", { name: "Changes" });

    // `unviewedFiles` is absent from the payload: nothing records per-file review progress, and a
    // zero here would claim every changed file had been reviewed.
    expect(screen.getAllByText("—").length).toBeGreaterThan(0);
  });

  it("shows each section's coverage rather than one banner", async () => {
    const report = emptySessionRunReport(SESSION);
    report.coverage.sections.usage = { state: "unavailable", reasonCodes: ["usage_unavailable"] };
    mount(<ReportTab sessionId={SESSION} service={service(report)} />);

    await screen.findByRole("heading", { name: "Usage" });
    // Eight complete, one not. A single report-level banner would either hide the gap or discredit
    // the eight sections that are fine.
    expect(screen.getAllByText(/Complete/).length).toBe(8);
    expect(screen.getByText(/Unavailable · usage_unavailable/)).toBeTruthy();
  });

  it("translates a refusal code rather than showing the backend's words", async () => {
    mount(
      <ReportTab
        sessionId={SESSION}
        service={service(new EvidenceUnavailableError("evidence_unavailable", "raw internals"))}
      />,
    );

    expect(await screen.findByText(/not available in this runtime/)).toBeTruthy();
    expect(screen.queryByText("raw internals")).toBeNull();
  });

  it("sends a section's evidence link to the tab that answers it", async () => {
    mount(
      <>
        <ActiveTabProbe />
        <ReportTab sessionId={SESSION} service={service(populated())} />
      </>,
    );
    await screen.findByRole("heading", { name: "Failures" });

    const links = screen.getAllByRole("button", { name: /Open evidence/ });
    await userEvent.click(links[links.length - 1]);

    // Asserted through the real provider rather than a stubbed context: a fake would prove the
    // panel calls something, not that what it calls moves the workspace. A reason code is a thing
    // the logs can be filtered by, which is why failures land there.
    expect(screen.getByTestId("active-tab").textContent).toBe("logs");
  });
});

/** Reads the tab the provider settled on, so a navigation is observable as its own effect. */
function ActiveTabProbe() {
  const { activeTab } = useWorkspaceEvidenceScope();
  return <output data-testid="active-tab">{activeTab}</output>;
}

/** Stands in for the conversation whose mounted length used to decide the report's contents. */
function MessageHarness({ children, count }: { children: ReactElement; count: number }) {
  return (
    <div>
      <ol data-testid="messages">
        {Array.from({ length: count }, (_, index) => (
          <li key={index}>message {index}</li>
        ))}
      </ol>
      {children}
    </div>
  );
}

/**
 * Every rendered figure, in document order.
 *
 * The property under test is that the whole page holds still, so the snapshot is the whole page's
 * numbers rather than one of them — a single value could stay put while its neighbours moved.
 */
function figures(): string {
  return Array.from(document.querySelectorAll("strong"))
    .map((node) => node.textContent ?? "")
    .join("|");
}
