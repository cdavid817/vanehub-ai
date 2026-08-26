// @vitest-environment jsdom

import { screen, waitFor, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { describe, expect, it, vi } from "vitest";
import "../../../i18n";
import { renderWithAppProviders } from "../../../test/render";
import type {
  AgentPersonalizationCapability,
  PersonalizationPolicyRef,
} from "../../../types/personalization";
import { isIncomplete, nextScope, PersonalizationScopeSelector } from "./scope-selector";
import type { WorkspaceOption } from "./use-scope-options";

/** Not in any shipped roster. A fixed set of checkboxes would simply not have a box for it. */
const AGENTS: AgentPersonalizationCapability[] = [
  {
    agentId: "synthetic-lab-agent",
    displayName: "Synthetic Lab Agent",
    supportsCustomInstructions: true,
    supportsMemoryIndex: true,
    supportsSelectedMemoryBodies: false,
    supportsAutomaticExtraction: false,
  },
];

const WORKSPACES: WorkspaceOption[] = [
  { workspaceKey: "ws-local-/code/vanehub", displayName: "vanehub", kind: "local" },
  { workspaceKey: "ws-remote-dev@build-01:22/srv/app", displayName: "build-01: app", kind: "remote" },
];

function renderSelector(scope: PersonalizationPolicyRef) {
  const onChange = vi.fn();
  renderWithAppProviders(
    <PersonalizationScopeSelector
      agents={AGENTS}
      onChange={onChange}
      scope={scope}
      workspaces={WORKSPACES}
    />,
  );
  return onChange;
}

describe("PersonalizationScopeSelector", () => {
  it("offers an Agent it was told about rather than one it was built with", async () => {
    renderSelector({ scopeKind: "agent", agentId: "synthetic-lab-agent" });

    const select = screen.getByTestId("personalization-scope-agent");
    expect(within(select).getByText("Synthetic Lab Agent")).toBeTruthy();
  });

  it("asks for a key only when the layer is named after one", async () => {
    renderSelector({ scopeKind: "global" });

    expect(screen.queryByTestId("personalization-scope-agent")).toBeNull();
    expect(screen.queryByTestId("personalization-scope-workspace")).toBeNull();
  });

  it("asks for both keys for a workspace-Agent layer", async () => {
    renderSelector({ scopeKind: "workspace-agent" });

    expect(screen.getByTestId("personalization-scope-agent")).toBeTruthy();
    expect(screen.getByTestId("personalization-scope-workspace")).toBeTruthy();
  });

  it("offers local and remote workspaces by name and reports the key", async () => {
    const onChange = renderSelector({ scopeKind: "workspace" });

    await userEvent.selectOptions(
      screen.getByTestId("personalization-scope-workspace"),
      "ws-remote-dev@build-01:22/srv/app",
    );

    // The user reads a name; the scope carries the key, because two remote paths can look alike
    // and must not merge into one layer.
    await waitFor(() => {
      expect(onChange).toHaveBeenCalledWith({
        scopeKind: "workspace",
        workspaceKey: "ws-remote-dev@build-01:22/srv/app",
      });
    });
  });

  it("says an incomplete scope is not ready to edit", async () => {
    renderSelector({ scopeKind: "workspace-agent", agentId: "synthetic-lab-agent" });

    expect(screen.getByTestId("personalization-scope-incomplete")).toBeTruthy();
  });

  it("stops saying so once every key is chosen", async () => {
    renderSelector({
      scopeKind: "workspace-agent",
      agentId: "synthetic-lab-agent",
      workspaceKey: "ws-local-/code/vanehub",
    });

    expect(screen.queryByTestId("personalization-scope-incomplete")).toBeNull();
  });

  it("drops a key the new layer does not use", () => {
    const carried = nextScope("workspace", {
      scopeKind: "workspace-agent",
      agentId: "synthetic-lab-agent",
      workspaceKey: "ws-local-/code/vanehub",
    });

    // Carrying the Agent id would address a layer the user cannot see on screen.
    expect(carried).toEqual({
      scopeKind: "workspace",
      agentId: undefined,
      workspaceKey: "ws-local-/code/vanehub",
    });
  });

  it("keeps a key the new layer still needs", () => {
    expect(nextScope("workspace-agent", { scopeKind: "agent", agentId: "synthetic-lab-agent" })).toEqual({
      scopeKind: "workspace-agent",
      agentId: "synthetic-lab-agent",
      workspaceKey: undefined,
    });
  });

  it.each([
    [{ scopeKind: "global" } as const, false],
    [{ scopeKind: "agent" } as const, true],
    [{ scopeKind: "agent", agentId: "a" } as const, false],
    [{ scopeKind: "workspace" } as const, true],
    [{ scopeKind: "workspace", workspaceKey: "w" } as const, false],
    [{ scopeKind: "workspace-agent", agentId: "a" } as const, true],
    [{ scopeKind: "workspace-agent", agentId: "a", workspaceKey: "w" } as const, false],
  ])("knows whether %o still needs a key", (scope, expected) => {
    expect(isIncomplete(scope)).toBe(expected);
  });
});
