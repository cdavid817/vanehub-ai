// @vitest-environment jsdom

import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { render, screen, waitFor } from "@testing-library/react";
import { readFileSync } from "node:fs";
import type { ReactElement } from "react";
import { I18nextProvider } from "react-i18next";
import { beforeAll, beforeEach, describe, expect, it, vi } from "vitest";
import { activateAppLanguage, i18n } from "../i18n";
import type { ChatMessage } from "../types/chat";

const { mockAgentService } = vi.hoisted(() => ({
  mockAgentService: {
    listSessionDirectory: vi.fn(),
    listSessionDocuments: vi.fn(),
    listSessionLogs: vi.fn(),
    readSessionFile: vi.fn(),
  },
}));

vi.mock("../services/runtime-agent-client", () => ({ agentService: mockAgentService }));

const { DocumentsTab } = await import("./documents-tab");
const { FilesTab } = await import("./files-tab");
const { LogsTab } = await import("./logs-tab");
const { ReportTab } = await import("./report-tab");

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

function message(content: string): ChatMessage {
  return {
    id: `message-${content}`,
    sessionId: "session-1",
    role: "assistant",
    content,
    status: "completed",
    toolUse: [],
    tokenUsage: { input: 1, output: 1 },
    createdAt: "2026-08-23T00:00:00.000Z",
    updatedAt: "2026-08-23T00:00:01.000Z",
    sessionSequence: 1,
    executionRunId: null,
  };
}

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

  it("holds the Report aggregation while hidden and catches up when shown", () => {
    const { rerenderPanel } = mount(<ReportTab isVisible messages={[message("one")]} partial={false} />);
    expect(screen.getByText("Message status")).toBeTruthy();

    rerenderPanel(<ReportTab isVisible={false} messages={[message("one"), message("two")]} partial={false} />);
    const heldStatuses = screen.getAllByText("1").length;

    rerenderPanel(<ReportTab isVisible messages={[message("one"), message("two")]} partial={false} />);

    // Held, not stale-forever: the same message list produces the newer aggregate once visible.
    expect(screen.getAllByText("2").length).toBeGreaterThan(0);
    expect(heldStatuses).toBeGreaterThan(0);
  });

  it("detaches the Shell view without ending the shell", () => {
    // A project-relative path rather than `import.meta.url`: under the jsdom environment that URL
    // is not a file URL, and `readFileSync` rejects it.
    const source = readFileSync("src/session-workspace/shell-tab.tsx", "utf8");
    const visibilityGuards = source.split("if (!visibleRef.current) return;").length - 1;

    // Both the input path and the resize observer stop while hidden.
    expect(visibilityGuards).toBe(2);
    expect(source).toContain("useEffect(() => { visibleRef.current = isVisible; }, [isVisible]);");
    // Nothing in the visibility path may kill: the process, its scrollback, and its working
    // directory outlive a glance at another tab. `killShell` stays in teardown and in the
    // explicit Disconnect button, both of which are user- or lifecycle-driven.
    const effectStart = source.indexOf("if (!isVisible) return;");
    const visibilityEffect = source.slice(effectStart, source.indexOf("}, [isVisible]);", effectStart));
    expect(effectStart).toBeGreaterThan(0);
    expect(visibilityEffect).not.toContain("killShell");
  });
});
