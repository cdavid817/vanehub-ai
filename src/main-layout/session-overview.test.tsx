// @vitest-environment jsdom

import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { fireEvent, render, screen, waitFor } from "@testing-library/react";
import { MemoryRouter } from "react-router";
import { afterEach, beforeAll, beforeEach, describe, expect, it, vi } from "vitest";
import { activateAppLanguage } from "../i18n";
import { agentService } from "../services/runtime-agent-client";
import type { Session } from "../types/agent";
import type { WorkbenchSelection } from "../types/workbench-selection";
import type { InspectorProviderContext } from "../ui/inspector/inspector-provider-registry";
import { SessionOverview } from "./session-overview";

/**
 * The data-fetching shell: resolves a `session`-kind selection against the shared `["sessions"]`
 * query and hands the result to `AsyncBoundary`. `SessionOverviewSections` (what renders once a
 * session resolves) has its own dedicated test file — this one stays at the loading/error/
 * unavailable/found boundary.
 */

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

function renderOverview(sessionId: string, context: InspectorProviderContext = {}) {
  const queryClient = new QueryClient({ defaultOptions: { queries: { retry: false } } });
  const selection: WorkbenchSelection = { kind: "session", sessionId };
  return render(
    <MemoryRouter>
      <QueryClientProvider client={queryClient}>
        <SessionOverview context={context} selection={selection} />
      </QueryClientProvider>
    </MemoryRouter>,
  );
}

beforeAll(async () => {
  await activateAppLanguage("en");
});

beforeEach(() => {
  // SessionOverviewSections' migrated panes each read on their own; none of that is this file's
  // concern, so every dependency they might touch is stubbed to a harmless rejection up front.
  vi.spyOn(agentService, "getWorkspaceEvidenceSummary").mockRejectedValue(new Error("unavailable"));
  vi.spyOn(agentService, "getWorkspaceInspectionCapabilities").mockRejectedValue(new Error("unavailable"));
  vi.spyOn(agentService, "getTokenUsageSummary").mockRejectedValue(new Error("unavailable"));
  vi.spyOn(agentService, "getSkillOverview").mockRejectedValue(new Error("unavailable"));
  vi.spyOn(agentService, "getSessionChatConfig").mockResolvedValue({ modelId: null, providerId: null } as never);
});

afterEach(() => {
  vi.restoreAllMocks();
});

describe("SessionOverview", () => {
  it("shows a loading state while the sessions list is still loading", () => {
    vi.spyOn(agentService, "listSessions").mockReturnValue(new Promise<Session[]>(() => {}));

    renderOverview("session-1");

    expect(screen.getByRole("status")).toBeTruthy();
  });

  it("shows a retryable error when the sessions list fails to load", async () => {
    const listSessions = vi.spyOn(agentService, "listSessions").mockRejectedValue(new Error("network down"));

    renderOverview("session-1");

    await waitFor(() => expect(screen.getByText("This session could not be loaded.")).toBeTruthy());
    expect(listSessions).toHaveBeenCalledTimes(1);

    fireEvent.click(screen.getByRole("button", { name: "Retry" }));
    await waitFor(() => expect(listSessions).toHaveBeenCalledTimes(2));
  });

  it("shows unavailable when the selected session id is not in the loaded list", async () => {
    vi.spyOn(agentService, "listSessions").mockResolvedValue([session({ id: "some-other-session" })]);

    renderOverview("session-1");

    await waitFor(() => expect(screen.getByText("Not available")).toBeTruthy());
  });

  it("renders the resolved session's overview once the list loads", async () => {
    vi.spyOn(agentService, "listSessions").mockResolvedValue([session()]);

    renderOverview("session-1");

    expect(await screen.findByRole("button", { name: "Runtime" })).toBeTruthy();
    expect(screen.getByText("CLI work")).toBeTruthy();
  });

  it("finds an archived-only session the same way it finds any other — listSessions is the only source of truth", async () => {
    // Documented, accepted limitation (see session-overview.tsx): a session absent from
    // `listSessions()` — archived-only or genuinely deleted — resolves to the same `unavailable`
    // state either way. This proves the *found* half stays correct for a session carrying
    // `archived: true`, since nothing here special-cases that field.
    vi.spyOn(agentService, "listSessions").mockResolvedValue([session({ archived: true })]);

    renderOverview("session-1");

    expect(await screen.findByRole("button", { name: "Runtime" })).toBeTruthy();
  });
});
