// @vitest-environment jsdom

import { screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { describe, expect, it, vi } from "vitest";
import "../../../i18n";
import { SettingsProvider } from "../../settings-provider";
import { createAgentServiceDouble, renderWithAppProviders } from "../../../test/render";
import type { PersonalizationPolicy, WorkspaceScopeInput } from "../../../types/personalization";
import type { SettingsPageStatus } from "../../settings-page-types";
import { PersonalizationInstructionsView } from "./instructions-view";

const GLOBAL_POLICY: PersonalizationPolicy = {
  scopeKind: "global",
  scopeKey: "",
  revision: 7,
  instructionMergeMode: "append",
  aboutUser: "Backend engineer.",
  styleRules: "Lead with the conclusion.",
  memoryReadMode: "enabled",
  explicitSaveMode: "enabled",
  automaticExtractionMode: "enabled",
  globalMemoryAccessMode: "enabled",
};

function renderView(
  overrides: Parameters<typeof createAgentServiceDouble>[0] = {},
  onStatusChange?: (status: SettingsPageStatus | null) => void,
) {
  const resolvePersonalizationWorkspace = vi.fn(async (input: WorkspaceScopeInput) =>
    input.projectPath
      ? { workspaceKey: `ws-local-${input.projectPath}`, kind: "local" as const }
      : { workspaceKey: `ws-remote-${input.remote?.host}`, kind: "remote" as const },
  );
  const service = createAgentServiceDouble({
    listPersonalizationAgentCapabilities: async () => [
      {
        agentId: "synthetic-lab-agent",
        displayName: "Synthetic Lab Agent",
        supportsCustomInstructions: true,
        supportsMemoryIndex: true,
        supportsSelectedMemoryBodies: false,
        supportsAutomaticExtraction: false,
      },
    ],
    listKnownProjects: async () => [
      { path: "/code/vanehub", displayName: "vanehub", isGit: true, lastOpenedAt: "2026-01-01T00:00:00Z" },
    ],
    listKnownRemoteWorkspaces: async () => [
      {
        host: "build-01",
        port: 22,
        user: "dev",
        path: "/srv/app",
        displayName: "build-01: app",
        uri: "ssh://dev@build-01:22/srv/app",
        lastOpenedAt: "2026-01-01T00:00:00Z",
      },
    ],
    resolvePersonalizationWorkspace,
    getPersonalizationPolicy: async (scope) => (scope.scopeKind === "global" ? GLOBAL_POLICY : null),
    ...overrides,
  });
  const rendered = renderWithAppProviders(
    <SettingsProvider>
      <PersonalizationInstructionsView onStatusChange={onStatusChange} service={service} />
    </SettingsProvider>,
  );
  return { ...rendered, resolvePersonalizationWorkspace };
}

describe("PersonalizationInstructionsView", () => {
  it("opens on the global layer and reports its stored revision", async () => {
    renderView();

    await waitFor(() => {
      expect(screen.getByTestId("personalization-scope-status").textContent).toContain("7");
    });
  });

  it("says a layer has never been written rather than showing it as empty", async () => {
    renderView();

    await screen.findByTestId("personalization-scope-kind");
    await userEvent.selectOptions(screen.getByTestId("personalization-scope-kind"), "agent");
    await userEvent.selectOptions(screen.getByTestId("personalization-scope-agent"), "synthetic-lab-agent");

    // Never written and written-to-all-inherit are different: the first has no revision to
    // conflict against, and the next save has to know which it is.
    await waitFor(() => {
      expect(screen.getByTestId("personalization-scope-status").textContent).toContain("从未写入过");
    });
  });

  it("does not read a layer whose keys are not all chosen", async () => {
    const getPersonalizationPolicy = vi.fn(async () => null);
    renderView({ getPersonalizationPolicy });

    await screen.findByTestId("personalization-scope-kind");
    // The initial global read is scheduled asynchronously; clearing before it fires would let it
    // land after the clear and read as a violation of the incomplete-scope rule below.
    await waitFor(() => {
      expect(getPersonalizationPolicy).toHaveBeenCalled();
    });
    getPersonalizationPolicy.mockClear();
    await userEvent.selectOptions(screen.getByTestId("personalization-scope-kind"), "workspace-agent");

    await waitFor(() => {
      expect(screen.getByTestId("personalization-scope-incomplete")).toBeTruthy();
    });
    expect(getPersonalizationPolicy).not.toHaveBeenCalled();
  });

  it("never sends a remote workspace as a URI", async () => {
    const { resolvePersonalizationWorkspace } = renderView();

    await waitFor(() => {
      expect(resolvePersonalizationWorkspace).toHaveBeenCalledTimes(2);
    });

    // A URI can carry `user:password@host`. The parts have nowhere to put one, which is stronger
    // than discarding a password after it has already crossed the boundary.
    for (const [input] of resolvePersonalizationWorkspace.mock.calls) {
      expect(JSON.stringify(input)).not.toContain("ssh://");
    }
    expect(resolvePersonalizationWorkspace).toHaveBeenCalledWith({
      remote: { host: "build-01", port: 22, user: "dev", path: "/srv/app" },
    });
  });

  it("lists a workspace by name while addressing it by key", async () => {
    renderView();

    await userEvent.selectOptions(await screen.findByTestId("personalization-scope-kind"), "workspace");
    const select = await screen.findByTestId("personalization-scope-workspace");

    await waitFor(() => {
      expect(select.querySelector('option[value="ws-local-/code/vanehub"]')?.textContent).toBe("vanehub");
    });
  });

  it("flags the shell status once the open layer's draft becomes dirty", async () => {
    const onStatusChange = vi.fn();
    const { user } = renderView({}, onStatusChange);

    await waitFor(() => {
      expect(onStatusChange).toHaveBeenCalledWith(null);
    });

    await user.type(await screen.findByTestId("personalization-field-aboutUser"), "!");

    await waitFor(() => {
      expect(onStatusChange).toHaveBeenCalledWith({
        kind: "unsaved",
        labelKey: "personalization.status.unsaved",
        labelParams: { count: 1 },
      });
    });
  });
});
