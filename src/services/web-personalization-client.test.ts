import { describe, expect, it } from "vitest";
import { webPersonalizationClient } from "./web-personalization-client";

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
