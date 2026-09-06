// @vitest-environment jsdom

import type { ComponentProps } from "react";
import { act, waitFor } from "@testing-library/react";
import { beforeEach, describe, expect, it, vi } from "vitest";
import { renderWithAppProviders } from "../test/render";
import type { AgentRegistryEntry, ProjectInspection } from "../types/agent";
import type { CreateSessionDialogContent as ContentComponent } from "./create-session-dialog-content";

type ContentProps = ComponentProps<typeof ContentComponent>;

const { agentService, sshConnectionService, rendered } = vi.hoisted(() => ({
  agentService: {
    listExpertRoles: vi.fn(),
    listKnownProjects: vi.fn(),
    listKnownRemoteWorkspaces: vi.fn(),
    inspectProject: vi.fn(),
    selectProjectDirectory: vi.fn(),
    createSession: vi.fn(),
    getSession: vi.fn(),
  },
  sshConnectionService: { listConnections: vi.fn() },
  rendered: [] as unknown[],
}));

vi.mock("../services/runtime-agent-client", () => ({ agentService }));
vi.mock("../services/runtime-ssh-connection-client", () => ({ sshConnectionService }));

// Stands in for the form so the assertions can read the values the dialog derives, rather than
// hunting for them through rendered markup that this test is not about.
vi.mock("./create-session-dialog-content", () => ({
  CreateSessionDialogContent: (props: unknown) => {
    rendered.push(props);
    return null;
  },
}));

import { CreateSessionDialog } from "./create-session-dialog";

const agent = {
  id: "codex-cli",
  displayName: "Codex CLI",
  provider: "OpenAI",
  launch: { kind: "cli" },
  supportedInteractionModes: ["cli"],
  availabilityState: "available",
  capabilityTags: ["cli"],
  agentOrigin: "builtin",
} satisfies AgentRegistryEntry;

function inspection(path: string, isGit: boolean): ProjectInspection {
  return { path, displayName: path, isGit, gitRoot: isGit ? path : null };
}

function latest(): ContentProps {
  return rendered.at(-1) as ContentProps;
}

/**
 * The dialog's own wiring, not just the tracker underneath it.
 *
 * The tracker is unit-tested separately. What this covers is that the dialog consults it — wiring
 * is where this class of fix is usually lost, because the helper is correct, the call site ignores
 * it, and every unit test still passes.
 */
describe("create-session dialog inspection wiring", () => {
  beforeEach(() => {
    rendered.length = 0;
    vi.clearAllMocks();
    agentService.listExpertRoles.mockResolvedValue([]);
    agentService.listKnownProjects.mockResolvedValue([]);
    agentService.listKnownRemoteWorkspaces.mockResolvedValue([]);
    sshConnectionService.listConnections.mockResolvedValue([]);
  });

  function mount() {
    renderWithAppProviders(
      <CreateSessionDialog
        agents={[agent]}
        onClose={() => {}}
        onConfigureOnePiece={() => {}}
        onCreated={() => {}}
        open
      />,
    );
  }

  it("does not let an earlier inspection decide a later path's Git capability", async () => {
    // A is a repository and resolves last; B is not and resolves first. This is the ordering the
    // audit reproduced: the second request is often on a warm cache while the first is still
    // walking a cold share.
    let resolveA: (value: ProjectInspection) => void = () => {};
    agentService.inspectProject.mockImplementation((path: string) =>
      path === "D:/a"
        ? new Promise<ProjectInspection>((resolve) => {
            resolveA = resolve;
          })
        : Promise.resolve(inspection("D:/b", false)),
    );

    mount();
    await waitFor(() => expect(latest()).toBeTruthy());

    await act(async () => {
      latest().onInspectPath("D:/a");
    });
    await act(async () => {
      latest().onInspectPath("D:/b");
    });
    await waitFor(() => expect(latest().projectPath).toBe("D:/b"));

    await act(async () => {
      resolveA(inspection("D:/a", true));
      await Promise.resolve();
    });

    // B is not a repository. If A's late result were applied, the dialog would offer worktree
    // creation for a folder that has no repository to branch from.
    expect(latest().projectPath).toBe("D:/b");
    expect(latest().gitCapable).toBe(false);
  });

  it("rejects a result for a folder the field has moved on from without a new inspection", async () => {
    // The path field updates on every keystroke and only inspects on blur. So the selected folder
    // can change with no new request in flight — and a request id alone cannot notice that.
    let resolveA: (value: ProjectInspection) => void = () => {};
    agentService.inspectProject.mockImplementation(
      () =>
        new Promise<ProjectInspection>((resolve) => {
          resolveA = resolve;
        }),
    );

    mount();
    await waitFor(() => expect(latest()).toBeTruthy());

    await act(async () => {
      latest().onInspectPath("D:/a");
    });
    // Typing, not blurring: no second inspection is started.
    await act(async () => {
      latest().setProjectPath("D:/b");
    });
    await waitFor(() => expect(latest().projectPath).toBe("D:/b"));

    await act(async () => {
      resolveA(inspection("D:/a", true));
      await Promise.resolve();
    });

    // A's answer describes a folder that is no longer selected. Applying it would offer worktree
    // creation for a repository the user is no longer pointing at.
    expect(latest().projectPath).toBe("D:/b");
    expect(latest().gitCapable).toBe(false);
  });

  it("keeps the current path's own result when it is the one that resolves", async () => {
    agentService.inspectProject.mockImplementation((path: string) =>
      Promise.resolve(inspection(path, path === "D:/b")),
    );

    mount();
    await waitFor(() => expect(latest()).toBeTruthy());

    await act(async () => {
      latest().onInspectPath("D:/b");
    });

    // The counterpart: attribution must not become "reject everything".
    await waitFor(() => expect(latest().gitCapable).toBe(true));
  });

  it("starts a reopened dialog with no inherited inspection", async () => {
    agentService.inspectProject.mockResolvedValue(inspection("D:/a", true));

    const { rerender } = renderWithAppProviders(
      <CreateSessionDialog
        agents={[agent]}
        onClose={() => {}}
        onConfigureOnePiece={() => {}}
        onCreated={() => {}}
        open
      />,
    );
    await waitFor(() => expect(latest()).toBeTruthy());
    await act(async () => {
      latest().onInspectPath("D:/a");
    });
    await waitFor(() => expect(latest().gitCapable).toBe(true));

    const props = { agents: [agent], onClose: () => {}, onConfigureOnePiece: () => {}, onCreated: () => {} };
    rerender(<CreateSessionDialog {...props} open={false} />);
    rerender(<CreateSessionDialog {...props} open />);

    // The path was cleared on reopen, so the capability derived from the old one must go with it.
    // Leaving it behind offered worktree creation for a folder the dialog no longer had.
    await waitFor(() => expect(latest().projectPath).toBe(""));
    expect(latest().gitCapable).toBe(false);
  });
});
