// @vitest-environment jsdom

import { fireEvent, render, screen, waitFor } from "@testing-library/react";
import { beforeAll, beforeEach, describe, expect, it, vi } from "vitest";
import { activateAppLanguage } from "../i18n";
import type { AgentRegistryEntry, KnownProject, KnownRemoteWorkspace, ProjectInspection } from "../types/agent";

const mocks = vi.hoisted(() => ({
  connections: vi.fn(),
  inspectProject: vi.fn(),
  knownProjects: vi.fn(),
  knownRemoteWorkspaces: vi.fn(),
  listExpertRoles: vi.fn(),
}));

vi.mock("../services/runtime-agent-client", () => ({
  agentService: {
    inspectProject: mocks.inspectProject,
    listExpertRoles: mocks.listExpertRoles,
    listKnownProjects: mocks.knownProjects,
    listKnownRemoteWorkspaces: mocks.knownRemoteWorkspaces,
  },
}));

vi.mock("../services/runtime-ssh-connection-client", () => ({
  sshConnectionService: { listConnections: mocks.connections },
}));

import { CreateSessionDialogContent } from "./create-session-dialog-content";
import { useCreateSessionDraft } from "./use-create-session-draft";
import type { CreateSessionWorkspacePrefill } from "./create-session-workspace-prefill";

const agent: AgentRegistryEntry = {
  id: "codex-cli", displayName: "Codex CLI", provider: "OpenAI",
  launch: { kind: "cli", executableName: "codex" }, supportedInteractionModes: ["cli"],
  availabilityState: "available", capabilityTags: [], agentOrigin: "builtin",
};
const localProject: KnownProject = { path: "D:\\code\\my-project", displayName: "my-project", isGit: true, lastOpenedAt: "2026-08-01T00:00:00.000Z" };
const remoteWorkspace: KnownRemoteWorkspace = {
  displayName: "dev.example.com:app", host: "dev.example.com", lastOpenedAt: "2026-08-01T00:00:00.000Z",
  path: "/work/app", port: 22, uri: "ssh://vane@dev.example.com/work/app", user: "vane",
};
const inspection: ProjectInspection = { displayName: "my-project", gitRoot: localProject.path, isGit: true, path: localProject.path };
// Stable references, not literals inline in Harness below: `useCreateSessionDraft`'s own reset
// effect depends on `agents` (via `availableAgents`), the same as `main-layout.tsx`'s real
// `model.agents` already is -- a fresh array/function identity on every Harness render would
// re-fire that effect (and the whole draft reset) forever instead of once.
const agents: AgentRegistryEntry[] = [agent];
const onCreated = vi.fn();

/**
 * Mirrors `CreateSessionDialog.tsx`'s own composition of `useCreateSessionDraft` +
 * `CreateSessionDialogContent`, without that file's own `open` gate -- this test always renders
 * with `open: true`, since the whole point is to exercise the model this hook produces and hand it
 * to the same content component the real dialog uses, not to re-test the open/close toggle.
 */
function Harness({ prefillWorkspace }: { prefillWorkspace?: CreateSessionWorkspacePrefill | null }) {
  const model = useCreateSessionDraft({ agents, onCreated, open: true, prefillWorkspace });
  return <CreateSessionDialogContent model={model} onClose={vi.fn()} onConfigureOnePiece={vi.fn()} />;
}

describe("create-session wizard prefill reaches Review without skipping steps (task 13.9)", () => {
  beforeAll(async () => {
    await activateAppLanguage("en");
  });

  beforeEach(() => {
    mocks.connections.mockReset().mockResolvedValue([]);
    mocks.inspectProject.mockReset().mockResolvedValue(inspection);
    mocks.knownProjects.mockReset().mockResolvedValue([localProject]);
    mocks.knownRemoteWorkspaces.mockReset().mockResolvedValue([remoteWorkspace]);
    mocks.listExpertRoles.mockReset().mockResolvedValue([]);
  });

  it("prefills a local workspace's path, then requires three explicit Next clicks to reach Review with it shown", async () => {
    render(<Harness prefillWorkspace={{ kind: "local", workspaceId: localProject.path }} />);

    // The wizard always mounts on step 1 regardless of prefill -- confirms prefilling only ever
    // changes field values, never the step index (see use-create-session-draft.ts's own comment).
    expect(screen.getByRole("heading", { name: "Session type" })).toBeTruthy();
    await waitFor(() => expect(mocks.inspectProject).toHaveBeenCalledWith(localProject.path));

    fireEvent.click(screen.getByRole("button", { name: "Next" }));
    expect(screen.getByRole("heading", { name: "Participants" })).toBeTruthy();

    fireEvent.click(screen.getByRole("button", { name: "Next" }));
    expect(screen.getByRole("heading", { name: "Workspace" })).toBeTruthy();
    expect(screen.getByDisplayValue(localProject.path)).toBeTruthy();

    fireEvent.click(screen.getByRole("button", { name: "Next" }));
    expect(screen.getByRole("heading", { name: "Review" })).toBeTruthy();
    expect(screen.getByText(localProject.path)).toBeTruthy();
    // Reaching Review with a valid, prefilled draft still leaves Create to the reader -- prefilling
    // never calls submit on its own behalf.
    expect(screen.getByRole("button", { name: "Create" }).hasAttribute("disabled")).toBe(false);
  });

  it("prefills a matched remote workspace's fields and reaches Review showing host:path, mirroring selectHistory's own field set", async () => {
    render(<Harness prefillWorkspace={{ kind: "ssh", workspaceId: remoteWorkspace.uri }} />);

    await waitFor(() => expect(mocks.knownRemoteWorkspaces).toHaveBeenCalled());

    fireEvent.click(screen.getByRole("button", { name: "Next" }));
    fireEvent.click(screen.getByRole("button", { name: "Next" }));
    expect(screen.getByRole("heading", { name: "Workspace" })).toBeTruthy();
    // findByDisplayValue (not getBy) because the prefill's `.then()` continuation may still be
    // pending when these two Next clicks (synchronous, local wizard-step state) land -- it retries
    // until the async dispatch from applyRemoteWorkspacePrefill has actually applied.
    expect(await screen.findByDisplayValue(remoteWorkspace.host)).toBeTruthy();
    expect(screen.getByDisplayValue(remoteWorkspace.path)).toBeTruthy();

    fireEvent.click(screen.getByRole("button", { name: "Next" }));
    expect(screen.getByRole("heading", { name: "Review" })).toBeTruthy();
    expect(screen.getByText(`${remoteWorkspace.host}:${remoteWorkspace.path}`)).toBeTruthy();
  });

  it("prefills nothing and stays on the wizard's own default state when the ssh id no longer matches any known remote workspace", async () => {
    mocks.knownRemoteWorkspaces.mockResolvedValue([]);
    render(<Harness prefillWorkspace={{ kind: "ssh", workspaceId: "ssh://vane@gone.example.com/app" }} />);

    await waitFor(() => expect(mocks.knownRemoteWorkspaces).toHaveBeenCalled());

    // Local/remote defaults to Local (createInitialCreateSessionDraft) when no match was found --
    // an unmatched id must not silently switch the mode with nothing behind it.
    fireEvent.click(screen.getByRole("button", { name: "Next" }));
    fireEvent.click(screen.getByRole("button", { name: "Next" }));
    expect(screen.getByPlaceholderText("D:\\\\code\\\\project")).toBeTruthy();
  });
});
