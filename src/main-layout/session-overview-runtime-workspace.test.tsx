// @vitest-environment jsdom

import type { ReactElement } from "react";
import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { render, screen } from "@testing-library/react";
import { afterEach, beforeAll, describe, expect, it, vi } from "vitest";
import { activateAppLanguage } from "../i18n";
import { agentService } from "../services/runtime-agent-client";
import type { Session } from "../types/agent";
import { SessionOverviewRuntimeSection, SessionOverviewWorkspaceSection } from "./session-overview-runtime-workspace";

function session(overrides: Partial<Session> = {}): Session {
  return {
    id: "session-1",
    title: "CLI work",
    agentId: "codex-cli",
    interactionMode: "cli",
    personalizationMode: "standard",
    lifecycleState: "running",
    recoveryStatus: "clean",
    recoveryRevision: 0,
    stateRevision: 0,
    historyRevision: 0,
    activeExecutionRunId: null,
    folder: "D:\\code\\vanehub-ai",
    projectPath: "D:\\code\\vanehub-ai",
    worktreePath: null,
    worktreeName: null,
    worktreeBranch: null,
    remoteWorkspace: null,
    remoteSshConnectionId: null,
    remoteSshConnectionRevision: null,
    runtimeSessionId: null,
    categoryId: null,
    pinned: false,
    archived: false,
    createdAt: "2026-08-27T00:00:00.000Z",
    updatedAt: "2026-08-27T00:00:00.000Z",
    ...overrides,
  };
}

function renderWithClient(ui: ReactElement) {
  const queryClient = new QueryClient({ defaultOptions: { queries: { retry: false } } });
  return render(<QueryClientProvider client={queryClient}>{ui}</QueryClientProvider>);
}

beforeAll(async () => {
  await activateAppLanguage("en");
});

afterEach(() => {
  vi.restoreAllMocks();
});

describe("SessionOverviewRuntimeSection", () => {
  it("shows the session title, CLI identity, and lifecycle immediately, and the resolved model once it loads", async () => {
    vi.spyOn(agentService, "getSessionChatConfig").mockResolvedValue({
      agentId: "codex-cli",
      interactionMode: "cli",
      executionMode: "execute",
      providerId: "openai",
      modelId: "gpt-5-5",
      reasoningDepth: "high",
      streaming: true,
      thinking: true,
      longContext: false,
    } as never);

    renderWithClient(<SessionOverviewRuntimeSection session={session()} />);

    expect(screen.getByText("CLI work")).toBeTruthy();
    expect(screen.getByText("Running")).toBeTruthy();
    expect(await screen.findByText("GPT-5.5")).toBeTruthy();
  });

  it("shows a no-model fallback rather than leaving the field blank", async () => {
    vi.spyOn(agentService, "getSessionChatConfig").mockResolvedValue({ modelId: null, providerId: null } as never);

    renderWithClient(<SessionOverviewRuntimeSection session={session()} />);

    expect(await screen.findByText("No model configured")).toBeTruthy();
  });
});

describe("SessionOverviewWorkspaceSection", () => {
  it("normalizes and displays the resolved workspace path", () => {
    vi.spyOn(agentService, "getWorkspaceEvidenceSummary").mockRejectedValue(new Error("unavailable"));
    vi.spyOn(agentService, "getWorkspaceInspectionCapabilities").mockRejectedValue(new Error("unavailable"));

    renderWithClient(
      <SessionOverviewWorkspaceSection
        active
        context={{}}
        displayPath={"\\\\?\\D:\\code\\vanehub-ai"}
        onShowUsage={vi.fn()}
        session={session()}
      />,
    );

    expect(screen.getByText("D:\\code\\vanehub-ai")).toBeTruthy();
  });

  it("shows a no-workspace fallback when there is no path to display", () => {
    vi.spyOn(agentService, "getWorkspaceEvidenceSummary").mockRejectedValue(new Error("unavailable"));
    vi.spyOn(agentService, "getWorkspaceInspectionCapabilities").mockRejectedValue(new Error("unavailable"));

    renderWithClient(
      <SessionOverviewWorkspaceSection active context={{}} displayPath={null} onShowUsage={vi.fn()} session={session()} />,
    );

    expect(screen.getByText("No project selected")).toBeTruthy();
  });

  it("only reads the evidence summary while this section is the active one", () => {
    const getSummary = vi.spyOn(agentService, "getWorkspaceEvidenceSummary").mockRejectedValue(new Error("unavailable"));
    vi.spyOn(agentService, "getWorkspaceInspectionCapabilities").mockRejectedValue(new Error("unavailable"));

    renderWithClient(
      <SessionOverviewWorkspaceSection active={false} context={{}} displayPath={"D:\\code\\vanehub-ai"} onShowUsage={vi.fn()} session={session()} />,
    );

    expect(screen.getByText("D:\\code\\vanehub-ai")).toBeTruthy();
    expect(getSummary).not.toHaveBeenCalled();
  });
});
