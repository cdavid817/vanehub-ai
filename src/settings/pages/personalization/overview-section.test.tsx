// @vitest-environment jsdom

import { screen, waitFor, within } from "@testing-library/react";
import { describe, expect, it } from "vitest";
import "../../../i18n";
import { createAgentServiceDouble, renderWithAppProviders } from "../../../test/render";
import type {
  AgentPersonalizationCapability,
  EffectivePreview,
} from "../../../types/personalization";
import { PersonalizationOverviewSection } from "./overview-section";

/**
 * The Agent list is built from the registry, and nothing in it knows which Agents exist.
 *
 * `synthetic-lab-agent` is here to prove that: it is not in any hard-coded list, has a capability
 * combination none of the shipped Agents has, and still gets a row that honours it. An Agent
 * registered after this shipped has to work the same way.
 */
const SYNTHETIC: AgentPersonalizationCapability = {
  agentId: "synthetic-lab-agent",
  displayName: "Synthetic Lab Agent",
  supportsCustomInstructions: true,
  supportsMemoryIndex: true,
  supportsSelectedMemoryBodies: false,
  supportsAutomaticExtraction: false,
};

const LIMITED: AgentPersonalizationCapability = {
  agentId: "text-only-agent",
  displayName: "Text Only Agent",
  supportsCustomInstructions: false,
  supportsMemoryIndex: false,
  supportsSelectedMemoryBodies: false,
  supportsAutomaticExtraction: false,
};

function previewFor(agentId: string): EffectivePreview {
  const capable = agentId === SYNTHETIC.agentId;
  return {
    revisionToken: `1:${agentId}:standard`,
    instructionMode: "append",
    includedInstructions: capable
      ? [
          {
            field: "about_user",
            scopeKind: "global",
            scopeKey: "",
            policyRevision: 4,
            mergeAction: "appended",
            redactedText: "Backend engineer.",
            characters: 17,
          },
        ]
      : [],
    excludedInstructions: capable
      ? []
      : [{ field: "about_user", scopeKind: "global", scopeKey: "", reason: "runtime_capability" }],
    memoryDelivery: capable ? "index_only" : "none",
    memoryRead: capable,
    explicitSave: capable,
    automaticExtraction: false,
    candidateCreation: capable,
    retrievalWrite: capable,
    eligibleMemoryCount: capable ? 2 : 0,
    consideredMemoryCount: 3,
    memoryExclusions: [],
    warnings: capable ? [] : ["unknown-agent"],
    approximateTokens: 6,
    knownCharacters: 24,
    selectedBodyBudgetMax: 5,
    excludedSurfaces: [],
    estimatorVersion: "test",
    cliInternalCompactionManaged: false,
  };
}

function renderSection(overrides: Parameters<typeof createAgentServiceDouble>[0] = {}) {
  const service = createAgentServiceDouble({
    listPersonalizationAgentCapabilities: async () => [SYNTHETIC, LIMITED],
    previewEffectivePersonalization: async (input) => previewFor(input.agentId),
    listPersonalizationPolicies: async () => [],
    ...overrides,
  });
  return renderWithAppProviders(<PersonalizationOverviewSection service={service} />);
}

describe("PersonalizationOverviewSection", () => {
  it("gives a row to an Agent it has never heard of", async () => {
    renderSection();

    const row = await screen.findByTestId(`personalization-overview-agent-${SYNTHETIC.agentId}`);

    expect(within(row).getByText("Synthetic Lab Agent")).toBeTruthy();
    expect(within(row).getByText(SYNTHETIC.agentId)).toBeTruthy();
  });

  it("never implies selected bodies are injected for an index-only resolution", async () => {
    renderSection();

    const delivery = await screen.findByTestId(
      `personalization-overview-delivery-${SYNTHETIC.agentId}`,
    );

    expect(delivery.textContent).toBe("仅索引");
  });

  it("says extraction is unsupported rather than switched off", async () => {
    renderSection();

    const row = await screen.findByTestId(`personalization-overview-agent-${SYNTHETIC.agentId}`);

    // The Agent declares no extraction capability, so the row states that instead of offering the
    // user a switch that would never take effect.
    expect(within(row).getAllByText("该 Agent 不支持").length).toBeGreaterThan(0);
    expect(within(row).queryByText("关闭")).toBeNull();
  });

  it("marks an Agent that takes no instructions at all", async () => {
    renderSection();

    const row = await screen.findByTestId(`personalization-overview-agent-${LIMITED.agentId}`);

    expect(within(row).getAllByText("该 Agent 不支持").length).toBe(3);
    expect(within(row).getByText("未应用任何内容")).toBeTruthy();
  });

  it("reports a warning the resolution produced", async () => {
    renderSection();

    const warnings = await screen.findByTestId("personalization-overview-warnings");

    expect(warnings.textContent).toContain("不在注册表中");
  });

  it("says the summary may be incomplete rather than showing zeros as fact", async () => {
    renderSection({
      listPersonalizationAgentCapabilities: async () => {
        throw new Error("personalization-storage-unavailable");
      },
    });

    await waitFor(() => {
      expect(screen.getByTestId("personalization-overview-error")).toBeTruthy();
    });
  });

  it("counts the Agents the registry reported, not a fixed roster", async () => {
    renderSection();

    const overview = await screen.findByTestId("personalization-overview");
    await waitFor(() => {
      expect(within(overview).getByText("2")).toBeTruthy();
    });
  });
});
