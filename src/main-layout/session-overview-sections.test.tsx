// @vitest-environment jsdom

import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { fireEvent, render, screen, waitFor, within } from "@testing-library/react";
import { MemoryRouter } from "react-router";
import { afterEach, beforeAll, beforeEach, describe, expect, it, vi } from "vitest";
import { activateAppLanguage } from "../i18n";
import { evidenceSessionIdSchema } from "../contracts/session-workspace-evidence-ids";
import { agentService } from "../services/runtime-agent-client";
import type { Session } from "../types/agent";
import type { WorkspaceEvidenceSummary } from "../types/session-workspace-evidence";
import type { InspectorProviderContext } from "../ui/inspector/inspector-provider-registry";
import { SessionOverviewSections } from "./session-overview-sections";

/**
 * The Accordion composition: which of the seven sections appear for which session, that each
 * renders its real migrated pane (not a placeholder), and the EvidenceLink rows added alongside
 * Skills and IM. Loading/error/unavailable belongs to session-overview.test.tsx, one layer up.
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

function evidenceSummary(overrides: Partial<WorkspaceEvidenceSummary> = {}): WorkspaceEvidenceSummary {
  return {
    changes: { changedFiles: 0, unviewedFiles: 0 },
    coverage: { reasonCodes: [], state: "complete", truncated: false },
    executionRecords: { failed: 0, running: 0 },
    generatedAt: "2026-08-27T00:00:00Z",
    logs: { newErrors: 0 },
    runState: { startedAt: undefined, status: "queued" },
    sessionId: evidenceSessionIdSchema.parse("session-1"),
    shells: { live: 0 },
    traces: { failed: 0, running: 0 },
    usage: { coverage: "unavailable", reportedTokens: undefined },
    verification: { failed: 0, passed: 0 },
    ...overrides,
  };
}

function capabilities() {
  return {
    gitDiff: { available: false },
    gitStatus: { available: false },
    listFiles: { available: true },
    provider: "local" as const,
    readTextFiles: { available: true },
    searchFiles: { available: true },
    watchMode: "polling" as const,
  };
}

function renderSections(activeSession: Session, context: InspectorProviderContext = {}) {
  const queryClient = new QueryClient({ defaultOptions: { queries: { retry: false } } });
  return render(
    <MemoryRouter>
      <QueryClientProvider client={queryClient}>
        <SessionOverviewSections context={context} session={activeSession} />
      </QueryClientProvider>
    </MemoryRouter>,
  );
}

let getSkillOverview: ReturnType<typeof vi.spyOn>;

beforeAll(async () => {
  await activateAppLanguage("en");
});

beforeEach(() => {
  vi.spyOn(agentService, "getWorkspaceEvidenceSummary").mockRejectedValue(new Error("unavailable"));
  vi.spyOn(agentService, "getWorkspaceInspectionCapabilities").mockRejectedValue(new Error("unavailable"));
  vi.spyOn(agentService, "getTokenUsageSummary").mockRejectedValue(new Error("unavailable"));
  getSkillOverview = vi.spyOn(agentService, "getSkillOverview").mockRejectedValue(new Error("unavailable"));
  vi.spyOn(agentService, "getSessionChatConfig").mockResolvedValue({ modelId: null, providerId: null } as never);
  vi.spyOn(agentService, "listCodeIndexWorkspaces").mockResolvedValue([]);
  vi.spyOn(agentService, "getRetrievalConfiguration").mockResolvedValue({ sourceProfileId: null, embeddingModel: null, automaticCodeIndexMode: "disabled" });
});

afterEach(() => {
  vi.restoreAllMocks();
});

describe("SessionOverviewSections", () => {
  it("opens Runtime and Workspace by default and keeps the rest closed", () => {
    renderSections(session());

    expect(screen.getByRole("button", { name: "Runtime" }).getAttribute("aria-expanded")).toBe("true");
    expect(screen.getByRole("button", { name: "Workspace" }).getAttribute("aria-expanded")).toBe("true");
    expect(screen.getByRole("button", { name: "Token Usage" }).getAttribute("aria-expanded")).toBe("false");
    expect(screen.getByRole("button", { name: "Skill" }).getAttribute("aria-expanded")).toBe("false");
    expect(screen.getByRole("button", { name: "IM" }).getAttribute("aria-expanded")).toBe("false");
  });

  it("hides Participants for a single-seat session", () => {
    renderSections(session());

    expect(screen.queryByRole("button", { name: "Member Information" })).toBeNull();
    expect(screen.queryByTestId("session-roster-editor")).toBeNull();
  });

  it("shows Participants for a session that has held more than one seat", () => {
    renderSections(session({
      seats: [
        { agentId: "codex-cli", roleId: "architect", joinedAt: "2026-08-27T00:00:00Z" },
        { agentId: "claude-code", roleId: "implementer", joinedAt: "2026-08-27T00:00:00Z" },
      ],
    }));

    expect(screen.getByRole("button", { name: "Member Information" })).toBeTruthy();
    expect(screen.getByTestId("session-roster-editor")).toBeTruthy();
  });

  it("hides Code Index for a non-OnePiece session with a workspace", () => {
    renderSections(session({ agentId: "codex-cli", worktreePath: "D:\\code\\wt" }));

    expect(screen.queryByRole("button", { name: "Code Index" })).toBeNull();
  });

  it("hides Code Index for a OnePiece session with no workspace path", () => {
    renderSections(session({ agentId: "onepiece", folder: null, projectPath: null, worktreePath: null }));

    expect(screen.queryByRole("button", { name: "Code Index" })).toBeNull();
  });

  it("shows Code Index, wired to the resolved workspace path, for a OnePiece session with one", async () => {
    renderSections(session({ agentId: "onepiece", worktreePath: "D:\\code\\wt" }));
    const header = screen.getByRole("button", { name: "Code Index" });

    fireEvent.click(header);

    // Real SessionCodeIndexPane content (workspacePath threaded through), not a placeholder.
    expect(await screen.findByText("Not indexed")).toBeTruthy();
  });

  it("renders each section's real migrated pane, not a placeholder", async () => {
    renderSections(session());

    // Runtime + Workspace: open by default, content already visible. Scoped to the Workspace
    // section's own content region — the (closed but mounted) Skills pane's own Project subview
    // renders this same path text, so an unscoped query would be ambiguous.
    expect(screen.getByText("CLI work")).toBeTruthy();
    const workspaceHeader = screen.getByRole("button", { name: "Workspace" });
    const workspaceContent = document.getElementById(workspaceHeader.getAttribute("aria-controls") ?? "");
    expect(within(workspaceContent as HTMLElement).getByText("D:\\code\\vanehub-ai")).toBeTruthy();

    fireEvent.click(screen.getByRole("button", { name: "Token Usage" }));
    await waitFor(() => expect(screen.getByText("Failed to load usage statistics: unavailable")).toBeTruthy());

    fireEvent.click(screen.getByRole("button", { name: "Skill" }));
    expect(screen.getByRole("tab", { name: /Effective/ })).toBeTruthy();

    fireEvent.click(screen.getByRole("button", { name: "IM" }));
    expect(screen.getByTestId("session-im-pane")).toBeTruthy();
  });

  it("gates each pane's own reads on whether its section is actually open", async () => {
    renderSections(session());
    expect(getSkillOverview).not.toHaveBeenCalled();

    fireEvent.click(screen.getByRole("button", { name: "Skill" }));

    await waitFor(() => expect(getSkillOverview).toHaveBeenCalled());
  });

  it("adds a Settings EvidenceLink beside the Skills pane's own settings affordance", () => {
    renderSections(session());
    fireEvent.click(screen.getByRole("button", { name: "Skill" }));

    const link = screen.getByRole("link", { name: "Open Skills settings" });
    expect(link.getAttribute("href")).toBe("/settings?section=skills");
  });

  it("adds a Settings EvidenceLink beside the IM pane's own settings affordance", () => {
    renderSections(session());
    fireEvent.click(screen.getByRole("button", { name: "IM" }));

    const link = screen.getByRole("link", { name: "Open IM settings" });
    expect(link.getAttribute("href")).toBe("/settings?section=im");
  });

  it("expands Usage locally when the Workspace section's evidence summary asks to show it", async () => {
    vi.spyOn(agentService, "getWorkspaceInspectionCapabilities").mockResolvedValue(capabilities());
    vi.spyOn(agentService, "getWorkspaceEvidenceSummary").mockResolvedValue(evidenceSummary());

    renderSections(session());
    const usageRow = await screen.findByRole("button", { name: /^Usage/ });

    fireEvent.click(usageRow);

    expect(screen.getByRole("button", { name: "Token Usage" }).getAttribute("aria-expanded")).toBe("true");
  });

  it("expands the section named by context.requestedSessionSection", () => {
    renderSections(session(), { requestedSessionSection: "im" });

    expect(screen.getByRole("button", { name: "IM" }).getAttribute("aria-expanded")).toBe("true");
  });

  it("does not fight a reader who manually collapses a section the request already opened", () => {
    const { rerender } = render(
      <MemoryRouter>
        <QueryClientProvider client={new QueryClient({ defaultOptions: { queries: { retry: false } } })}>
          <SessionOverviewSections context={{ requestedSessionSection: "im" }} session={session()} />
        </QueryClientProvider>
      </MemoryRouter>,
    );
    fireEvent.click(screen.getByRole("button", { name: "IM" }));
    expect(screen.getByRole("button", { name: "IM" }).getAttribute("aria-expanded")).toBe("false");

    // An unrelated re-render with the same (never-reset) requested value must not reopen it.
    rerender(
      <MemoryRouter>
        <QueryClientProvider client={new QueryClient({ defaultOptions: { queries: { retry: false } } })}>
          <SessionOverviewSections context={{ requestedSessionSection: "im" }} session={session({ title: "Renamed" })} />
        </QueryClientProvider>
      </MemoryRouter>,
    );
    expect(screen.getByRole("button", { name: "IM" }).getAttribute("aria-expanded")).toBe("false");
  });

  it("ignores an unknown requested section rather than throwing", () => {
    expect(() => renderSections(session(), { requestedSessionSection: "not-a-real-section" })).not.toThrow();
  });
});
