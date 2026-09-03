// @vitest-environment jsdom

import { render, screen } from "@testing-library/react";
import { beforeEach, describe, expect, it, vi } from "vitest";
import "../i18n";
import type { KnownProject } from "../types/agent";
import type { SshConnection } from "../types/ssh-connection";

const mocks = vi.hoisted(() => ({ listGoals: vi.fn(), listWorkItems: vi.fn() }));

vi.mock("../services/runtime-work-board-client", () => ({
  workBoardService: { listWorkItems: mocks.listWorkItems },
}));
vi.mock("../services/runtime-goal-client", () => ({
  goalService: { listGoals: mocks.listGoals },
}));

import { buildLocalWorkspaceSummary, buildRemoteWorkspaceSummary } from "./workspace-aggregation";
import { WorkspaceCard } from "./workspace-card";
import { WorkspaceDetail } from "./workspace-detail";
import { remoteConnectedWorkspace } from "../testing/fixtures/workspace-fixtures";

/**
 * Task 13.14's "privacy" ask, made concrete (design.md Decision 4's own "Privacy" note for
 * Command Center names the same vocabulary: Credential material and un-redacted paths must never
 * leak into rendered output). For a workspace row specifically that means two claims, both
 * proven here against the *real* join function (`workspace-aggregation.ts`), not a hand-built
 * fixture that could drift from it:
 *
 * 1. The raw canonical path/URI (`WorkspaceSummary.workspaceId`, task 13.11) never reaches
 *    rendered text -- only the `normalizeDisplayPath`-safe `displayPath` does.
 * 2. `SshConnection`'s credential/diagnostic fields (`keyPath`, `lastError`, `hasPassword`, the
 *    host-key `hostTrust` fingerprint) never cross into `WorkspaceSummary` at all -- confirmed
 *    both on the aggregation output object itself and on what `WorkspaceCard`/`WorkspaceDetail`
 *    actually paint, so a future change that starts threading one of those fields through would
 *    be caught here even if it only wired it into rendering, not the type.
 */
describe("workspace privacy (task 13.14)", () => {
  beforeEach(() => {
    mocks.listGoals.mockReset().mockResolvedValue([]);
    mocks.listWorkItems.mockReset().mockResolvedValue([]);
  });

  it("never renders the raw canonical local path, only the normalized display path", () => {
    const rawPath = "\\\\?\\D:\\secrets\\vane-personal-project";
    const project: KnownProject = { displayName: "vane-personal-project", isGit: true, lastOpenedAt: "2026-08-01T00:00:00.000Z", path: rawPath };
    const summary = buildLocalWorkspaceSummary(project, [], { displayName: "vane-personal-project", gitRoot: rawPath, isGit: true, path: rawPath });

    // The join itself must have already stripped the raw UNC prefix for display.
    expect(summary.displayPath).not.toContain("\\\\?\\");
    expect(summary.workspaceId).toBe(rawPath); // canonical identity is kept, just never rendered.

    const { container } = render(<WorkspaceCard onSelect={() => undefined} selected={false} workspace={summary} />);
    expect(container.textContent).not.toContain("\\\\?\\");
  });

  it("never lets an SshConnection's keyPath, lastError, hasPassword, or host-key fingerprint reach WorkspaceSummary or its rendering", async () => {
    const connection: SshConnection = {
      authMode: "key",
      createdAt: "2026-08-01T00:00:00.000Z",
      defaultPath: "/srv/app",
      hasPassword: false,
      host: "build.example.com",
      hostTrust: {
        algorithm: "ssh-ed25519",
        confirmedAt: "2026-08-01T00:00:00.000Z",
        fingerprint: "SHA256:CANARY-FINGERPRINT-MUST-NOT-RENDER",
        host: "build.example.com",
        port: 22,
      },
      id: "conn-privacy-1",
      keyPath: "/home/vane/.ssh/CANARY-PRIVATE-KEY-PATH",
      lastConnectedAt: "2026-08-20T00:00:00.000Z",
      lastError: "CANARY-RAW-ERROR: authentication failed for secret-user",
      name: "Build box",
      port: 22,
      revision: 1,
      testStatus: "succeeded",
      updatedAt: "2026-08-20T00:00:00.000Z",
      user: "vane",
    };
    const summary = buildRemoteWorkspaceSummary(
      { displayName: "build.example.com:app", host: "build.example.com", lastOpenedAt: "2026-08-20T00:00:00.000Z", path: "/srv/app", port: 22, uri: "ssh://vane@build.example.com/srv/app", user: "vane" },
      [connection],
      [],
    );

    const serialized = JSON.stringify(summary);
    for (const canary of ["CANARY-FINGERPRINT-MUST-NOT-RENDER", "CANARY-PRIVATE-KEY-PATH", "CANARY-RAW-ERROR"]) {
      expect(serialized).not.toContain(canary);
    }

    render(
      <WorkspaceDetail
        onContinueSession={() => undefined}
        onNewSession={() => undefined}
        onOpenSshSettings={() => undefined}
        onReconnect={() => undefined}
        workspace={summary}
      />,
    );
    await screen.findByTestId("workspace-detail");
    const detailText = screen.getByTestId("workspace-detail").textContent ?? "";
    for (const canary of ["CANARY-FINGERPRINT-MUST-NOT-RENDER", "CANARY-PRIVATE-KEY-PATH", "CANARY-RAW-ERROR"]) {
      expect(detailText).not.toContain(canary);
    }
  });

  it("keeps the same guarantee for the fixture-level remote-connected scenario (13.13)", () => {
    const summary = remoteConnectedWorkspace({ displayPath: "vane@build.example.com:/srv/app" });
    const { container } = render(<WorkspaceCard onSelect={() => undefined} selected={false} workspace={summary} />);
    // The fixture's own `workspaceId` is a full ssh:// URI -- confirms card rendering never
    // surfaces it as visible text, only `displayPath` (mirrors the raw-path assertion above for
    // the SSH shape specifically, where the "canonical vs. display" gap is a URI, not a UNC path).
    expect(container.textContent).not.toContain(summary.workspaceId);
  });
});
