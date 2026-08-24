// @vitest-environment jsdom
import { beforeEach, describe, expect, it, vi } from "vitest";
import type { CliParameterService } from "./cli-service";
import { cliParameterCatalogVersion } from "./cli-parameter-registry";

const v1Key = "vanehub.cli-parameter-profiles.v1";
const v2Key = "vanehub.cli-parameter-profiles.v2";

// The adapter keeps an in-session memory copy, which is what makes the legacy migration run once
// rather than on every read. Each test therefore needs a fresh module instance, not just cleared
// storage — otherwise one test's memory copy is the next test's "already migrated".
let webCliParameterClient: CliParameterService;

async function codexProfile() {
  const profiles = await webCliParameterClient.listCliParameterProfiles();
  return profiles.find((profile) => profile.agentId === "codex-cli")!;
}

describe("web CLI parameter client", () => {
  beforeEach(async () => {
    localStorage.clear();
    vi.resetModules();
    ({ webCliParameterClient } = await import("./web-cli-parameter-client"));
  });

  it("starts every profile inherited, at revision 0, and never claims an installation", async () => {
    const codex = await codexProfile();

    expect(codex.revision).toBe(0);
    expect(codex.updatedAt).toBeNull();
    expect(codex.installation).toEqual({ installed: false, runnable: false, conflict: false });
    expect(Object.values(codex.selections).every((entry) => entry.state === "inherit")).toBe(true);
  });

  it("restores a saved profile across a reload", async () => {
    const before = await codexProfile();
    await webCliParameterClient.saveCliParameterProfile({
      agentId: "codex-cli",
      expectedRevision: before.revision,
      catalogVersion: before.catalogVersion,
      selections: { ...before.selections, model: { state: "value", value: "gpt-5.5" } },
    });

    // A reload sees only what browser storage kept, not the module's in-memory copy.
    const raw = localStorage.getItem(v2Key);
    expect(raw).not.toBeNull();
    const reloaded = JSON.parse(raw!) as Record<string, { selections: Record<string, unknown>; revision: number }>;
    expect(reloaded["codex-cli"].revision).toBe(1);
    expect(reloaded["codex-cli"].selections.model).toEqual({ state: "value", value: "gpt-5.5" });
  });

  it("rejects a stale revision and a stale catalog with distinct codes", async () => {
    const before = await codexProfile();
    await webCliParameterClient.saveCliParameterProfile({
      agentId: "codex-cli",
      expectedRevision: before.revision,
      catalogVersion: before.catalogVersion,
      selections: before.selections,
    });

    await expect(
      webCliParameterClient.saveCliParameterProfile({
        agentId: "codex-cli",
        expectedRevision: before.revision,
        catalogVersion: before.catalogVersion,
        selections: before.selections,
      }),
    ).rejects.toMatchObject({ code: "CLI_PARAMETER_REVISION_CONFLICT" });

    await expect(
      webCliParameterClient.saveCliParameterProfile({
        agentId: "codex-cli",
        expectedRevision: 1,
        catalogVersion: "0.0.1",
        selections: before.selections,
      }),
    ).rejects.toMatchObject({ code: "CLI_PARAMETER_CATALOG_MISMATCH" });
  });

  it("falls back to defaults when browser storage holds malformed JSON", async () => {
    localStorage.setItem(v2Key, "{not json");

    const codex = await codexProfile();

    expect(codex.revision).toBe(0);
    expect(codex.catalogVersion).toBe(cliParameterCatalogVersion);
  });

  it("migrates v1 rows by definition, not by string match, and leaves the v1 key intact", async () => {
    localStorage.setItem(
      v1Key,
      JSON.stringify({
        "codex-cli": {
          // v1's two "not set" sentinels.
          model: "default",
          ephemeral: false,
          // A real value, and an id the v2 registry no longer exposes to the page.
          reasoningEffort: "high",
          sandbox: "read-only",
        },
      }),
    );

    const codex = await codexProfile();

    expect(codex.selections.model).toEqual({ state: "inherit" });
    expect(codex.selections.ephemeral).toEqual({ state: "inherit" });
    expect(codex.selections.reasoningEffort).toEqual({ state: "value", value: "high" });
    expect(codex.selections.sandbox).toBeUndefined();
    expect(codex.revision).toBe(0);
    expect(localStorage.getItem(v1Key)).not.toBeNull();
  });

  it("does not re-run the migration once a v2 profile exists", async () => {
    localStorage.setItem(v1Key, JSON.stringify({ "codex-cli": { reasoningEffort: "high" } }));
    const migrated = await codexProfile();
    await webCliParameterClient.saveCliParameterProfile({
      agentId: "codex-cli",
      expectedRevision: migrated.revision,
      catalogVersion: migrated.catalogVersion,
      selections: { ...migrated.selections, reasoningEffort: { state: "inherit" } },
    });

    const after = await codexProfile();

    expect(after.selections.reasoningEffort).toEqual({ state: "inherit" });
    expect(after.revision).toBe(1);
  });

  it("resets to the registry defaults and keeps counting revisions", async () => {
    const before = await codexProfile();
    const saved = await webCliParameterClient.saveCliParameterProfile({
      agentId: "codex-cli",
      expectedRevision: before.revision,
      catalogVersion: before.catalogVersion,
      selections: { ...before.selections, model: { state: "value", value: "gpt-5.5" } },
    });

    const reset = await webCliParameterClient.resetCliParameterProfile({
      agentId: "codex-cli",
      expectedRevision: saved.revision,
      catalogVersion: saved.catalogVersion,
    });

    expect(reset.selections.model).toEqual({ state: "inherit" });
    expect(reset.revision).toBe(saved.revision + 1);
    expect(reset.savedPreviews.chat.global).toEqual([]);
  });

  it("refuses an unknown agent and an unknown parameter with located codes", async () => {
    const before = await codexProfile();

    await expect(
      webCliParameterClient.saveCliParameterProfile({
        agentId: "not-an-agent" as never,
        expectedRevision: 0,
        catalogVersion: cliParameterCatalogVersion,
        selections: {},
      }),
    ).rejects.toMatchObject({ code: "CLI_PARAMETER_UNKNOWN_AGENT" });

    await expect(
      webCliParameterClient.saveCliParameterProfile({
        agentId: "codex-cli",
        expectedRevision: before.revision,
        catalogVersion: before.catalogVersion,
        selections: { ...before.selections, nonesuch: { state: "value", value: true } },
      }),
    ).rejects.toMatchObject({
      code: "CLI_PARAMETER_UNKNOWN_PARAMETER",
      agentId: "codex-cli",
      parameterId: "nonesuch",
    });
  });
});
