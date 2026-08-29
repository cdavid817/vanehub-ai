// @vitest-environment jsdom

import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { render, screen, waitFor } from "@testing-library/react";
import { readFileSync } from "node:fs";
import type { ReactElement } from "react";
import { I18nextProvider } from "react-i18next";
import { beforeAll, beforeEach, describe, expect, it, vi } from "vitest";
import { activateAppLanguage, i18n } from "../i18n";
import type { EvidenceSessionId } from "../types/session-workspace-evidence";

const { mockAgentService } = vi.hoisted(() => ({
  mockAgentService: {
    listSessionDirectory: vi.fn(),
    listSessionDocuments: vi.fn(),
    getSessionRunReport: vi.fn(),
    listSessionLogs: vi.fn(),
    readSessionFile: vi.fn(),
  },
}));

vi.mock("../services/runtime-agent-client", () => ({ agentService: mockAgentService }));

const { DocumentsTab } = await import("./documents-tab");
const { FilesTab } = await import("./files-tab");
const { LogsTab } = await import("./logs-tab");
const { ReportTab } = await import("./report-tab");
const { WorkspaceEvidenceScopeProvider } = await import("./workspace-evidence-scope");
const { emptySessionRunReport } = await import("./report-test-fixtures");
const report = emptySessionRunReport("session-1");

function mount(ui: ReactElement) {
  const queryClient = new QueryClient({
    defaultOptions: { mutations: { retry: false }, queries: { retry: false } },
  });
  const wrap = (element: ReactElement) => (
    <I18nextProvider i18n={i18n}>
      <QueryClientProvider client={queryClient}>{element}</QueryClientProvider>
    </I18nextProvider>
  );
  const rendered = render(wrap(ui));
  return { rerenderPanel: (next: ReactElement) => rendered.rerender(wrap(next)) };
}

const directory = {
  items: [{ name: "main.rs", path: "main.rs", kind: "file" as const, size: 12 }],
  truncated: false,
  nextCursor: null,
  context: { availability: "available" as const, rootName: "project", reason: null },
  path: "",
};


describe("hidden workspace panels", () => {
  beforeAll(async () => {
    await activateAppLanguage("en");
  });

  beforeEach(() => {
    vi.clearAllMocks();
  });

  it("keeps the Files listing on screen while it stops re-reading the tree", async () => {
    mockAgentService.listSessionDirectory.mockResolvedValue(directory);
    const { rerenderPanel } = mount(<FilesTab isVisible sessionId="session-1" />);

    await waitFor(() => expect(screen.getByText("main.rs")).toBeTruthy());
    const readsWhileVisible = mockAgentService.listSessionDirectory.mock.calls.length;

    rerenderPanel(<FilesTab isVisible={false} sessionId="session-1" />);

    // The cache is what makes this safe to hide: the rows the user was reading are still there.
    expect(screen.getByText("main.rs")).toBeTruthy();
    expect(mockAgentService.listSessionDirectory.mock.calls.length).toBe(readsWhileVisible);
  });

  it("keeps the Documents list while the discovery walk stops", async () => {
    mockAgentService.listSessionDocuments.mockResolvedValue({
      items: [{ path: "README.md", kind: "markdown" as const }],
      truncated: false,
    });
    mockAgentService.readSessionFile.mockResolvedValue({
      path: "README.md",
      status: "text" as const,
      content: "# Title",
    });
    const { rerenderPanel } = mount(<DocumentsTab isVisible sessionId="session-1" />);

    await waitFor(() => expect(screen.getByText("README.md")).toBeTruthy());
    const walksWhileVisible = mockAgentService.listSessionDocuments.mock.calls.length;

    rerenderPanel(<DocumentsTab isVisible={false} sessionId="session-1" />);

    expect(screen.getByText("README.md")).toBeTruthy();
    expect(mockAgentService.listSessionDocuments.mock.calls.length).toBe(walksWhileVisible);
  });

  it("defers a Logs read taken while hidden until the panel is on screen", async () => {
    mockAgentService.listSessionLogs.mockResolvedValue({
      items: [{ id: "log-1", level: "error" as const, message: "boom", timestamp: "2026-08-23T00:00:00.000Z" }],
      nextCursor: null,
      truncated: false,
    });
    const { rerenderPanel } = mount(<LogsTab isVisible={false} sessionId="session-1" />);

    // Nothing is read for a panel nobody is looking at.
    expect(mockAgentService.listSessionLogs).not.toHaveBeenCalled();

    rerenderPanel(<LogsTab isVisible sessionId="session-1" />);

    // The read it owed is issued the moment it is visible, so the panel is never left showing
    // rows from a filter the user has since changed.
    await waitFor(() => expect(mockAgentService.listSessionLogs).toHaveBeenCalledTimes(1));
  });

  it("reads no report while hidden and issues one the moment it is shown", async () => {
    mockAgentService.getSessionRunReport.mockResolvedValue(report);
    const { rerenderPanel } = mount(
      <WorkspaceEvidenceScopeProvider seatIds={[]} sessionId={"session-1" as EvidenceSessionId}>
        <ReportTab isVisible={false} sessionId="session-1" />
      </WorkspaceEvidenceScopeProvider>,
    );

    // The report is a five-context read. A panel nobody is looking at must not pay for it.
    expect(mockAgentService.getSessionRunReport).not.toHaveBeenCalled();

    rerenderPanel(
      <WorkspaceEvidenceScopeProvider seatIds={[]} sessionId={"session-1" as EvidenceSessionId}>
        <ReportTab isVisible sessionId="session-1" />
      </WorkspaceEvidenceScopeProvider>,
    );

    await waitFor(() => expect(mockAgentService.getSessionRunReport).toHaveBeenCalledTimes(1));
  });

  it("detaches the Shell view without ending the shell", () => {
    // A project-relative path rather than `import.meta.url`: under the jsdom environment that URL
    // is not a file URL, and `readFileSync` rejects it.
    const surface = readFileSync("src/session-workspace/shell-surface.tsx", "utf8");
    const tab = readFileSync("src/session-workspace/shell-tab.tsx", "utf8");

    // Hiding now releases the attachment outright rather than guarding each path that would have
    // used it, which is why the guards this test used to count are gone: there is nothing left to
    // guard once the claim is released.
    expect(surface).toContain("if (!isVisible || !terminal) return;");
    expect(surface).toContain("void detach?.();");
    // Nothing in the visibility path may end a Shell: the process, its scrollback, and its working
    // directory outlive a glance at another tab. Closing is reachable only through the tab's
    // confirmation dialog, which is user-driven.
    expect(surface).not.toContain("closeSessionShell");
    expect(tab).toContain("ShellCloseDialog");
  });
});
