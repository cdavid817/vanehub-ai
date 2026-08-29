import { describe, expect, it } from "vitest";
import type {
  AgentPersonalizationCapability,
  EffectivePreview,
  PersonalizationPolicy,
} from "../../../types/personalization";
import { agentOverviewRow, overviewTotals, overviewWarnings } from "./overview-model";

function capability(
  overrides: Partial<AgentPersonalizationCapability> = {},
): AgentPersonalizationCapability {
  return {
    agentId: "onepiece",
    displayName: "OnePiece",
    supportsCustomInstructions: true,
    supportsMemoryIndex: true,
    supportsSelectedMemoryBodies: true,
    supportsAutomaticExtraction: true,
    ...overrides,
  };
}

function preview(overrides: Partial<EffectivePreview> = {}): EffectivePreview {
  return {
    revisionToken: "3:onepiece:standard",
    instructionMode: "append",
    includedInstructions: [
      {
        field: "about_user",
        scopeKind: "global",
        scopeKey: "",
        policyRevision: 3,
        mergeAction: "appended",
        redactedText: "Backend engineer.",
        characters: 17,
      },
    ],
    excludedInstructions: [],
    memoryDelivery: "index_with_selected_bodies",
    memoryRead: true,
    explicitSave: true,
    automaticExtraction: true,
    candidateCreation: true,
    retrievalWrite: true,
    eligibleMemoryCount: 2,
    consideredMemoryCount: 3,
    memoryExclusions: [],
    warnings: [],
    approximateTokens: 10,
    knownCharacters: 40,
    selectedBodyBudgetMax: 5,
    excludedSurfaces: [],
    estimatorVersion: "test",
    cliInternalCompactionManaged: false,
    ...overrides,
  };
}

function policy(overrides: Partial<PersonalizationPolicy> = {}): PersonalizationPolicy {
  return {
    scopeKind: "global",
    scopeKey: "",
    revision: 3,
    instructionMergeMode: "append",
    aboutUser: "abc",
    styleRules: "de",
    memoryReadMode: "enabled",
    explicitSaveMode: "enabled",
    automaticExtractionMode: "enabled",
    globalMemoryAccessMode: "enabled",
    ...overrides,
  };
}

describe("overview model", () => {
  it("tells apart a switch that is off and a runtime that cannot do it", () => {
    const unable = agentOverviewRow(
      capability({ supportsAutomaticExtraction: false }),
      preview({ automaticExtraction: false }),
    );
    const able = agentOverviewRow(capability(), preview({ automaticExtraction: false }));

    // The two look identical in a boolean and mean opposite things: one is a switch the user can
    // flip, the other is a fact about the Agent.
    expect(unable.extraction).toEqual({ kind: "unavailable", reason: "runtime_capability" });
    expect(able.extraction).toEqual({ kind: "off" });
  });

  it("reports what memory delivery resolved to, not what the Agent could accept", () => {
    const row = agentOverviewRow(
      capability({ supportsSelectedMemoryBodies: true }),
      preview({ memoryDelivery: "index_only" }),
    );

    // Claiming bodies were injected because the Agent could accept them would be a lie about this
    // resolution -- the policy is what decided it.
    expect(row.delivery).toBe("index_only");
  });

  it("does not imply instructions are applied for an Agent that cannot take them", () => {
    const row = agentOverviewRow(
      capability({ supportsCustomInstructions: false }),
      preview({ includedInstructions: [] }),
    );

    expect(row.instructions).toEqual({ kind: "unavailable", reason: "runtime_capability" });
    expect(row.sources).toEqual([]);
    expect(row.characters).toBe(0);
  });

  it("names every layer that contributed, once each and in order", () => {
    const row = agentOverviewRow(
      capability(),
      preview({
        includedInstructions: [
          {
            field: "about_user",
            scopeKind: "global",
            scopeKey: "",
            policyRevision: 3,
            mergeAction: "appended",
            redactedText: "a",
            characters: 1,
          },
          {
            field: "style_rules",
            scopeKind: "global",
            scopeKey: "",
            policyRevision: 3,
            mergeAction: "appended",
            redactedText: "bb",
            characters: 2,
          },
          {
            field: "style_rules",
            scopeKind: "agent",
            scopeKey: "onepiece",
            policyRevision: 5,
            mergeAction: "replaced",
            redactedText: "ccc",
            characters: 3,
          },
        ],
      }),
    );

    expect(row.sources).toEqual([
      { scopeKind: "global", scopeKey: "", revision: 3 },
      { scopeKind: "agent", scopeKey: "onepiece", revision: 5 },
    ]);
    expect(row.characters).toBe(6);
  });

  it("counts a layer the user wrote even when it resolves to all-inherit", () => {
    const totals = overviewTotals(
      [],
      [policy(), policy({ scopeKind: "agent", scopeKey: "onepiece", instructionMergeMode: "inherit", aboutUser: "", styleRules: "" })],
    );

    // A count that hid it would make an empty scope indistinguishable from one never opened.
    expect(totals.configuredScopes).toBe(2);
    expect(totals.globalCharacters).toBe(5);
  });

  it("counts only Agents whose state is actually on", () => {
    const totals = overviewTotals(
      [
        agentOverviewRow(capability(), preview()),
        agentOverviewRow(
          capability({ agentId: "gemini-cli", supportsMemoryIndex: false, supportsAutomaticExtraction: false, supportsCustomInstructions: false }),
          preview({ includedInstructions: [], memoryRead: false, automaticExtraction: false }),
        ),
      ],
      [policy()],
    );

    expect(totals).toMatchObject({
      agents: 2,
      agentsWithInstructions: 1,
      memoryReadAgents: 1,
      extractionAgents: 1,
    });
  });

  it("reports each warning cause once across all Agents", () => {
    const rows = [
      agentOverviewRow(capability(), preview({ warnings: ["migration-incomplete", "unknown-agent"] })),
      agentOverviewRow(capability({ agentId: "b" }), preview({ warnings: ["migration-incomplete"] })),
    ];

    expect(overviewWarnings(rows)).toEqual(["migration-incomplete", "unknown-agent"]);
  });
});
