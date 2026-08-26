// @vitest-environment jsdom

import { screen, waitFor, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { describe, expect, it, vi } from "vitest";
import "../../../i18n";
import { createAgentServiceDouble, renderWithAppProviders } from "../../../test/render";
import type { EffectivePreview } from "../../../types/personalization";
import { RuntimePreviewSection } from "./runtime-preview-section";

function preview(overrides: Partial<EffectivePreview> = {}): EffectivePreview {
  return {
    revisionToken: "1:synthetic-lab-agent:standard",
    instructionMode: "append",
    includedInstructions: [
      {
        field: "about_user",
        scopeKind: "global",
        scopeKey: "",
        policyRevision: 4,
        mergeAction: "appended",
        redactedText: "Backend engineer.",
        characters: 17,
      },
    ],
    excludedInstructions: [
      { field: "style_rules", scopeKind: "agent", scopeKey: "synthetic-lab-agent", reason: "empty_field" },
    ],
    memoryDelivery: "index_only",
    memoryRead: true,
    explicitSave: true,
    automaticExtraction: false,
    candidateCreation: true,
    retrievalWrite: true,
    eligibleMemoryCount: 2,
    consideredMemoryCount: 5,
    memoryExclusions: [{ reason: "other_workspace", count: 3 }],
    warnings: [],
    approximateTokens: 12,
    knownCharacters: 48,
    selectedBodyBudgetMax: 5,
    excludedSurfaces: [],
    estimatorVersion: "test",
    cliInternalCompactionManaged: false,
    ...overrides,
  };
}

function renderPreview(overrides: Parameters<typeof createAgentServiceDouble>[0] = {}) {
  const previewEffectivePersonalization = vi.fn(async () => preview());
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
    listKnownRemoteWorkspaces: async () => [],
    resolvePersonalizationWorkspace: async () => ({ workspaceKey: "ws-1", kind: "local" as const }),
    previewEffectivePersonalization,
    ...overrides,
  });
  const rendered = renderWithAppProviders(<RuntimePreviewSection service={service} />);
  return { ...rendered, previewEffectivePersonalization };
}

async function chooseAgent() {
  const select = await screen.findByTestId("personalization-preview-agent");
  await within(select).findByText("Synthetic Lab Agent");
  await userEvent.selectOptions(select, "synthetic-lab-agent");
}


describe("RuntimePreviewSection", () => {
  it("resolves nothing until an Agent is chosen", async () => {
    const world = renderPreview();
    await screen.findByTestId("personalization-preview-inputs");

    // A preview needs something to be about; guessing an Agent would show a resolution nobody asked
    // for.
    expect(screen.getByTestId("personalization-runtime-preview-empty")).toBeTruthy();
    expect(world.previewEffectivePersonalization).not.toHaveBeenCalled();
  });

  it("sends the selected Agent, workspace and session mode", async () => {
    const world = renderPreview();
    await chooseAgent();
    await userEvent.selectOptions(screen.getByTestId("personalization-preview-workspace"), "ws-1");
    await userEvent.selectOptions(screen.getByTestId("personalization-preview-mode"), "temporary");

    await waitFor(() => {
      expect(world.previewEffectivePersonalization).toHaveBeenCalledWith(
        expect.objectContaining({
          agentId: "synthetic-lab-agent",
          workspaceKey: "ws-1",
          sessionMode: "temporary",
        }),
      );
    });
  });

  it("names the layer and revision each applied instruction came from", async () => {
    renderPreview();
    await chooseAgent();

    const included = await screen.findByTestId("personalization-preview-included");
    expect(included.textContent).toContain("全局");
    expect(included.textContent).toContain("修订号 4");
    expect(included.textContent).toContain("Backend engineer.");
  });

  it("says what was left out and why", async () => {
    renderPreview();
    await chooseAgent();

    const excluded = await screen.findByTestId("personalization-preview-excluded");
    expect(excluded.textContent).toContain("该字段为空");
  });

  it("reports the memory counts and why the rest were excluded", async () => {
    renderPreview();
    await chooseAgent();

    await waitFor(() => {
      expect(screen.getByTestId("personalization-preview-counts").textContent).toContain("5");
    });
    expect(screen.getByTestId("personalization-preview-delivery").textContent).toBe("仅索引");
    expect(screen.getByTestId("personalization-preview-memory-exclusions").textContent).toContain(
      "属于另一个工作区",
    );
  });

  it("states that a CLI's own compaction is not VaneHub's, before and after a resolution", async () => {
    renderPreview();

    // Present without any selection at all: a user reading the estimate has to know what it does
    // not cover regardless of whether they have resolved anything yet.
    expect(screen.getByTestId("personalization-preview-cli-compaction").textContent).toContain(
      "VaneHub",
    );

    await chooseAgent();
    await screen.findByTestId("personalization-preview-output");
    expect(screen.getByTestId("personalization-preview-cli-compaction").textContent).toContain(
      "VaneHub",
    );
  });

  it("surfaces a warning the resolution produced", async () => {
    renderPreview({
      previewEffectivePersonalization: async () => preview({ warnings: ["no-validated-policy"] }),
    });
    await chooseAgent();

    const warnings = await screen.findByTestId("personalization-preview-warnings");
    expect(warnings.textContent).toContain("没有可用的已校验策略");
  });

  it("says the resolution is unreadable rather than showing an empty one", async () => {
    renderPreview({
      previewEffectivePersonalization: async () => {
        throw new Error("personalization-storage-unavailable");
      },
    });
    await chooseAgent();

    await waitFor(() => {
      expect(screen.getByTestId("personalization-preview-error")).toBeTruthy();
    });
    expect(screen.queryByTestId("personalization-preview-output")).toBeNull();
  });
});
