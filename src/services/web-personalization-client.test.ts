import { describe, expect, it } from "vitest";
import { webPersonalizationClient } from "./web-personalization-client";
import { previewFor } from "./web-personalization-preview";
import { listWebMemories, readWebPolicy } from "./web-personalization-state";
import type { PersonalizationPolicy } from "../types/personalization";
import type { MemoryDetail } from "../types/personalization-memory";

/**
 * The mock's job is to refuse the same things the desktop refuses.
 *
 * `CommandError` serializes to its message and nothing else, so the message string is the entire
 * contract a screen matches on. These assert the strings, not just that something threw.
 */

const client = webPersonalizationClient;

describe("web personalization mock", () => {
  it("refuses a policy write whose caller was looking at an older revision", async () => {
    const current = await client.getPersonalizationPolicy({ scopeKind: "global" });
    expect(current).not.toBeNull();

    await expect(
      client.patchPersonalizationPolicy({
        scopeKind: "global",
        expectedRevision: (current?.revision ?? 0) - 1,
        aboutUser: "stale write",
      }),
    ).rejects.toThrow(/personalization-revision-conflict: expected \d+, stored \d+/u);
  });

  it("advances the revision on a write the caller was up to date for", async () => {
    const before = await client.getPersonalizationPolicy({ scopeKind: "agent", agentId: "onepiece" });
    const written = await client.patchPersonalizationPolicy({
      scopeKind: "agent",
      agentId: "onepiece",
      expectedRevision: before?.revision ?? 0,
      styleRules: "Prefer short answers.",
    });

    expect(written.revision).toBe((before?.revision ?? 0) + 1);
    expect(written.styleRules).toBe("Prefer short answers.");
    // An untouched field keeps its stored value rather than being republished as empty.
    expect(written.aboutUser).toBe(before?.aboutUser ?? "");
  });

  it("refuses a scope that is missing the key it is named after", async () => {
    await expect(client.getPersonalizationPolicy({ scopeKind: "agent" })).rejects.toThrow(
      "unsupported policy scope: agent",
    );
  });

  it("never puts a memory body in a list entry", async () => {
    const page = await client.queryPersonalizationMemories({});

    expect(page.items.length).toBeGreaterThan(0);
    for (const item of page.items) {
      expect(item).not.toHaveProperty("content");
    }
  });

  it("pages through with a cursor it issued rather than restarting at the top", async () => {
    const first = await client.queryPersonalizationMemories({ limit: 1 });
    expect(first.nextCursor).not.toBeNull();

    const second = await client.queryPersonalizationMemories({
      limit: 1,
      cursor: first.nextCursor ?? undefined,
    });

    expect(second.items[0]?.id).not.toBe(first.items[0]?.id);
  });

  it("refuses a cursor it did not issue", async () => {
    await expect(
      client.queryPersonalizationMemories({ cursor: "mem-0000000000000001" }),
    ).rejects.toThrow("unreadable page cursor");
  });

  it("delivers no memory at all in a temporary session", async () => {
    const preview = await client.previewEffectivePersonalization({
      agentId: "onepiece",
      sessionId: "session-1",
      workspaceKey: "ws-vanehub",
      sessionMode: "temporary",
    });

    expect(preview.eligibleMemoryCount).toBe(0);
    expect(preview.memoryDelivery).toBe("none");
    expect(preview.automaticExtraction).toBe(false);
    // The count of what was considered still reports, so a screen can say why nothing was used.
    expect(preview.consideredMemoryCount).toBeGreaterThan(0);
    expect(preview.memoryExclusions.some((entry) => entry.reason === "temporary_session")).toBe(true);
  });

  it("reports an allowed empty pool as allowed rather than as disabled", async () => {
    // A workspace nobody has saved anything for: reading is permitted, nothing is eligible, and
    // the mock must say both -- inferring the switch from the count is the bug this replaces.
    const preview = await client.previewEffectivePersonalization({
      agentId: "onepiece",
      sessionId: "session-1",
      workspaceKey: "ws-nothing-saved-here",
      sessionMode: "project-only",
    });

    expect(preview.memoryReadAllowed).toBe(true);
    expect(preview.memoryRead).toBe(true);
    expect(preview.readBlockReason).toBeNull();
    expect(preview.eligibleMemoryCount).toBe(0);
    expect(preview.memoryDelivery).toBe("index_with_selected_bodies");
    expect(preview.previewKind).toBe("hypothetical");
  });

  it("applies the Agent layer's read switch on top of the global one", () => {
    // The whole policy stack, not the global row alone: an Agent-level `disabled` narrows what the
    // global `enabled` granted, and the preview reports the block by its reason. Resolved over an
    // explicit policy list because the mock store keeps nothing outside a browser.
    const global = readWebPolicy({ scopeKind: "global" });
    expect(global).not.toBeNull();
    const agentLayer: PersonalizationPolicy = {
      scopeKind: "agent",
      scopeKey: "onepiece",
      revision: 1,
      instructionMergeMode: "inherit",
      aboutUser: "",
      styleRules: "",
      memoryReadMode: "disabled",
      explicitSaveMode: "inherit",
      automaticExtractionMode: "inherit",
      globalMemoryAccessMode: "inherit",
    };
    const input = { agentId: "onepiece", sessionId: "session-1", workspaceKey: "ws-vanehub" };

    const narrowed = previewFor(input, [global!, agentLayer], listWebMemories());
    expect(narrowed.memoryReadAllowed).toBe(false);
    expect(narrowed.readBlockReason).toBe("read_disabled");
    expect(narrowed.memoryDelivery).toBe("none");
    expect(narrowed.recallAvailability).toBe("disabled");
    expect(narrowed.memoryExclusions.some((entry) => entry.reason === "memory_read_disabled")).toBe(true);

    // The workspace-Agent layer sits above the Agent layer and can re-enable what it disabled.
    const workspaceAgentLayer: PersonalizationPolicy = {
      ...agentLayer,
      scopeKind: "workspace-agent",
      scopeKey: "ws-vanehub::onepiece",
      memoryReadMode: "enabled",
    };
    const widened = previewFor(input, [global!, agentLayer, workspaceAgentLayer], listWebMemories(), {
      retrievalConfigured: true,
    });
    expect(widened.memoryReadAllowed).toBe(true);
    expect(widened.recallAvailability).toBe("available");
  });

  it("admits a selected audience only to the Agents it names", () => {
    const global = readWebPolicy({ scopeKind: "global" });
    const restricted: MemoryDetail = {
      ...listWebMemories()[0]!,
      id: "mem-0000000000000099",
      scopeKind: "global",
      workspaceKey: null,
      status: "active",
      audienceAgentIds: ["claude-code"],
    };
    const previewAs = (agentId: string) =>
      previewFor({ agentId, sessionId: "session-1", workspaceKey: "ws-vanehub" }, [global!], [restricted]);

    expect(previewAs("onepiece").memoryExclusions).toEqual([{ reason: "agent_audience", count: 1 }]);
    expect(previewAs("claude-code").eligibleMemoryCount).toBe(1);
  });

  it("drops global memories in a project-only session", async () => {
    const preview = await client.previewEffectivePersonalization({
      agentId: "onepiece",
      sessionId: "session-1",
      workspaceKey: "ws-vanehub",
      sessionMode: "project-only",
    });

    expect(preview.memoryExclusions.some((entry) => entry.reason === "project_only_session")).toBe(
      true,
    );
  });

  it("reports what an Agent cannot consume rather than pretending it did", async () => {
    const preview = await client.previewEffectivePersonalization({
      agentId: "gemini-cli",
      sessionId: "session-1",
    });

    expect(preview.includedInstructions).toHaveLength(0);
    expect(
      preview.excludedInstructions.every((segment) => segment.reason === "runtime_capability"),
    ).toBe(true);
    expect(preview.cliInternalCompactionManaged).toBe(false);
  });

  it("refuses a reset whose token was issued for a different scope", async () => {
    const preview = await client.previewPersonalizationReset({
      scopeKind: "global",
      includeArchived: false,
    });

    await expect(
      client.executePersonalizationReset(
        { scopeKind: "any", includeArchived: false },
        preview.confirmationToken,
        "DELETE",
      ),
    ).rejects.toThrow("personalization-reset-refused: token-scope-mismatch");
  });

  it("refuses a reset the user did not type the phrase for", async () => {
    const scope = { scopeKind: "global" as const, includeArchived: false };
    const preview = await client.previewPersonalizationReset(scope);

    await expect(
      client.executePersonalizationReset(scope, preview.confirmationToken, "delete"),
    ).rejects.toThrow("personalization-reset-refused: phrase-mismatch");
  });

  it("refuses a memory update whose caller was looking at an older revision", async () => {
    const created = await client.createPersonalizationMemory({
      name: "conflict-probe",
      description: "Written so the stale write below has something to miss.",
      memoryType: "user",
      content: "A memory used only by this test.",
      scopeKind: "global",
    });

    await expect(
      client.updatePersonalizationMemory({
        id: created.id,
        expectedRevision: created.revision + 1,
        content: "Written from a stale copy.",
      }),
    ).rejects.toThrow(/personalization-revision-conflict/u);
  });

  it("refuses a memory whose content is only whitespace", async () => {
    await expect(
      client.createPersonalizationMemory({
        name: "blank",
        description: "",
        memoryType: "user",
        content: "   ",
        scopeKind: "global",
      }),
    ).rejects.toThrow("memory content must not be empty");
  });

  it("reports a maintenance result with no failures rather than omitting the field", async () => {
    const result = await client.reconcilePersonalizationMemories();

    expect(result.failures).toEqual([]);
    expect(result.quarantined).toBe(0);
  });
});
