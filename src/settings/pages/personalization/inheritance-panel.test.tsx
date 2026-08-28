// @vitest-environment jsdom

import { screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { describe, expect, it } from "vitest";
import "../../../i18n";
import { createAgentServiceDouble, renderWithAppProviders } from "../../../test/render";
import type { PersonalizationPolicy } from "../../../types/personalization";
import { PersonalizationInstructionsView } from "./instructions-view";

const GLOBAL: PersonalizationPolicy = {
  scopeKind: "global",
  scopeKey: "",
  revision: 3,
  instructionMergeMode: "append",
  aboutUser: "Backend engineer.",
  styleRules: "Lead with the conclusion.",
  memoryReadMode: "enabled",
  explicitSaveMode: "enabled",
  automaticExtractionMode: "enabled",
  globalMemoryAccessMode: "enabled",
};

const AGENT_LAYER: PersonalizationPolicy = {
  ...GLOBAL,
  scopeKind: "agent",
  scopeKey: "synthetic-lab-agent",
  revision: 2,
  instructionMergeMode: "inherit",
  aboutUser: "",
  styleRules: "Answer in Chinese.",
};

function renderView(overrides: Parameters<typeof createAgentServiceDouble>[0] = {}) {
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
    listKnownProjects: async () => [],
    listKnownRemoteWorkspaces: async () => [],
    listPersonalizationPolicies: async () => [GLOBAL, AGENT_LAYER],
    getPersonalizationPolicy: async (scope) =>
      scope.scopeKind === "global" ? GLOBAL : scope.scopeKind === "agent" ? AGENT_LAYER : null,
    ...overrides,
  });
  return renderWithAppProviders(<PersonalizationInstructionsView service={service} />);
}

async function selectAgentScope() {
  await userEvent.selectOptions(await screen.findByTestId("personalization-scope-kind"), "agent");
  await userEvent.selectOptions(
    await screen.findByTestId("personalization-scope-agent"),
    "synthetic-lab-agent",
  );
}

describe("inheritance panel", () => {
  it("says the global layer is where inheritance starts", async () => {
    renderView();

    await waitFor(() => {
      expect(screen.getByTestId("personalization-inheritance-none")).toBeTruthy();
    });
  });

  it("shows the text of the layer below, with its revision", async () => {
    renderView();
    await selectAgentScope();

    const layer = await screen.findByTestId("personalization-inherited-global");
    expect(layer.textContent).toContain("Backend engineer.");
    expect(layer.textContent).toContain("3");
  });

  it("says what append will do before the save, not after", async () => {
    renderView();
    await selectAgentScope();

    await userEvent.selectOptions(await screen.findByTestId("personalization-merge-mode"), "append");
    await userEvent.type(await screen.findByTestId("personalization-field-aboutUser"), "More.");

    // Append and replace look identical in the field afterwards; the only way to find out which
    // happened would be to start a session.
    await waitFor(() => {
      expect(screen.getByTestId("personalization-merge-outcome").textContent).toContain("接在下方");
    });
  });

  it("says replace uses the text instead of the layers below", async () => {
    renderView();
    await selectAgentScope();

    await userEvent.type(await screen.findByTestId("personalization-field-aboutUser"), "Mine.");
    await userEvent.selectOptions(await screen.findByTestId("personalization-merge-mode"), "replace");

    await waitFor(() => {
      expect(screen.getByTestId("personalization-merge-outcome").textContent).toContain("取代下方");
    });
  });

  it("says a disabled layer keeps its text and leaves the layers below applying", async () => {
    renderView();
    await selectAgentScope();

    await userEvent.selectOptions(await screen.findByTestId("personalization-merge-mode"), "disabled");

    await waitFor(() => {
      expect(screen.getByTestId("personalization-merge-outcome").textContent).toContain("照常生效");
    });
  });

  it("does not promise an effect for a layer with no text", async () => {
    renderView();
    await selectAgentScope();

    await userEvent.selectOptions(await screen.findByTestId("personalization-merge-mode"), "append");
    await userEvent.clear(await screen.findByTestId("personalization-field-aboutUser"));
    await userEvent.clear(await screen.findByTestId("personalization-field-styleRules"));

    await waitFor(() => {
      expect(screen.getByTestId("personalization-merge-outcome").textContent).toContain("不贡献任何内容");
    });
  });
});
