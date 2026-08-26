import { readFileSync, readdirSync } from "node:fs";
import { beforeEach, describe, expect, it, vi } from "vitest";

const invoked: { command: string; args?: Record<string, unknown> }[] = [];

vi.mock("@tauri-apps/api/core", () => ({
  invoke: (command: string, args?: Record<string, unknown>) => {
    invoked.push({ command, args });
    return Promise.resolve(null);
  },
}));

const { tauriPersonalizationClient } = await import("../services/tauri-personalization-client");
const { webPersonalizationClient } = await import("../services/web-personalization-client");

/**
 * Proves the two runtimes and the native command layer agree on names.
 *
 * TypeScript cannot see across the FFI boundary: a mistyped `invoke` command string, a wire field
 * the mock spells differently, or a native DTO that grew a field all typecheck perfectly and fail
 * only at runtime on a real desktop. So this reads the Rust sources and compares.
 */

/**
 * Reads with line endings normalised.
 *
 * The repo checks out CRLF on Windows, so a pattern anchored on a bare newline matches
 * nothing there -- and an empty parse makes every comparison below pass vacuously.
 */
function readNative(path: string): string {
  return readFileSync(path, "utf8").split("\r\n").join("\n");
}

const DTO_SOURCE = readNative("src-tauri/src/commands/personalization/dto.rs");
const COMMAND_DIR = "src-tauri/src/commands/personalization";
const REGISTRY = readNative("src-tauri/src/commands/supplemental_registry.rs");

function camel(value: string): string {
  return value.replace(/_([a-z])/gu, (_match, letter: string) => letter.toUpperCase());
}

/** Field names of every `#[serde(rename_all = "camelCase")]` struct in `dto.rs`, already camelised. */
function nativeViewFields(source: string): Map<string, string[]> {
  const views = new Map<string, string[]>();
  const pattern = /pub\(crate\) struct (\w+) \{([\s\S]*?)\n\}/gu;
  for (const [, name, body] of source.matchAll(pattern)) {
    const fields = [...body.matchAll(/^\s{4}pub\(crate\) (\w+):/gmu)].map(([, field]) => camel(field));
    views.set(name, fields);
  }
  return views;
}

/** Parameter names of every `#[tauri::command]`, minus the injected `api` state handle. */
function nativeCommandParameters(): Map<string, string[]> {
  const commands = new Map<string, string[]>();
  for (const file of readdirSync(COMMAND_DIR)) {
    if (!file.endsWith(".rs")) continue;
    const source = readNative(`${COMMAND_DIR}/${file}`);
    const match = /#\[tauri::command\]\npub\(crate\) fn (\w+)\(([\s\S]*?)\n\) ->/u.exec(source);
    if (!match) continue;
    const [, name, signature] = match;
    const parameters = [...signature.matchAll(/^\s{4}(\w+):/gmu)]
      .map(([, parameter]) => parameter)
      .filter((parameter) => parameter !== "api")
      .map(camel);
    commands.set(name, parameters);
  }
  return commands;
}

const views = nativeViewFields(DTO_SOURCE);
const commands = nativeCommandParameters();

function fieldsOf(view: string): string[] {
  const fields = views.get(view);
  if (!fields) throw new Error(`dto.rs has no struct named ${view}`);
  return [...fields].sort();
}

function keysOf(value: unknown): string[] {
  return Object.keys(value as Record<string, unknown>).sort();
}

async function captureInvocations(): Promise<Map<string, Record<string, unknown> | undefined>> {
  invoked.length = 0;
  const client = tauriPersonalizationClient;
  await client.getPersonalizationHealth();
  await client.listPersonalizationPolicies();
  await client.getPersonalizationPolicy({ scopeKind: "agent", agentId: "onepiece" });
  await client.patchPersonalizationPolicy({ scopeKind: "global", aboutUser: "x" });
  await client.previewEffectivePersonalization({ agentId: "onepiece", sessionId: "s-1" });
  await client.listPersonalizationAgentCapabilities();
  await client.resolvePersonalizationWorkspace({ projectPath: "D:/code/vanehub" });
  await client.queryPersonalizationMemories({ limit: 10 });
  await client.getPersonalizationMemory("mem-0000000000000001");
  await client.createPersonalizationMemory({
    name: "n",
    description: "d",
    memoryType: "user",
    content: "c",
    scopeKind: "global",
  });
  await client.updatePersonalizationMemory({ id: "mem-0000000000000001", expectedRevision: 1 });
  await client.deletePersonalizationMemory("mem-0000000000000001", 1);
  await client.listPersonalizationCandidates(10);
  await client.reviewPersonalizationCandidate({ candidateId: "cnd-1", action: "approve" });
  await client.previewPersonalizationReset({ includeArchived: false });
  await client.executePersonalizationReset({ includeArchived: false }, "token", "DELETE");
  await client.reconcilePersonalizationMemories();
  return new Map(invoked.map((entry) => [entry.command, entry.args]));
}

describe("personalization wire contract", () => {
  beforeEach(() => {
    invoked.length = 0;
  });

  it("still understands the Rust sources it reads", () => {
    // Without this, a parser that silently stopped matching would make every check below vacuous.
    expect(views.size).toBeGreaterThan(15);
    expect(commands.size).toBe(17);
    expect(fieldsOf("MemoryDetailView")).toContain("workspaceKey");
  });

  it("invokes only commands the native registry routes", async () => {
    const captured = await captureInvocations();

    expect(captured.size).toBe(17);
    for (const command of captured.keys()) {
      expect(commands.has(command)).toBe(true);
      // Registration alone is not enough: `registry.rs` routes by name, and a command missing from
      // `is_command` answers `Command <name> not found` at runtime.
      expect(REGISTRY).toContain(`"${command}"`);
    }
  });

  it("sends argument names the native signatures declare", async () => {
    const captured = await captureInvocations();

    for (const [command, parameters] of commands) {
      const args = captured.get(command);
      const sent = args ? Object.keys(args) : [];
      expect(sent.sort()).toEqual([...parameters].sort());
    }
  });

  it("sends payload fields the native input DTOs declare", async () => {
    const captured = await captureInvocations();
    const payloads: [string, string][] = [
      ["patch_personalization_policy", "PersonalizationPolicyPatchInput"],
      ["preview_effective_personalization", "EffectivePreviewInput"],
      ["query_personalization_memories", "MemoryQueryInput"],
      ["create_personalization_memory", "CreateMemoryCommandInput"],
      ["update_personalization_memory", "UpdateMemoryCommandInput"],
      ["review_personalization_candidate", "ReviewCandidateInput"],
      ["preview_personalization_reset", "ResetScopeInput"],
      ["resolve_personalization_workspace", "WorkspaceScopeInput"],
      ["execute_personalization_reset", "ResetScopeInput"],
    ];

    for (const [command, view] of payloads) {
      const input = captured.get(command)?.input as Record<string, unknown>;
      // A subset, not an equality: an optional field a caller left out is absent from the payload
      // and serde supplies its default. A field the DTO never declared is the real error.
      expect(fieldsOf(view)).toEqual(expect.arrayContaining(Object.keys(input)));
    }
  });

  it("returns the same field set from the mock that the native views serialize", async () => {
    const client = webPersonalizationClient;

    expect(keysOf(await client.getPersonalizationHealth())).toEqual(
      fieldsOf("PersonalizationHealthView"),
    );
    expect(keysOf((await client.listPersonalizationPolicies())[0])).toEqual(
      fieldsOf("PersonalizationPolicyView"),
    );
    expect(keysOf((await client.listPersonalizationAgentCapabilities())[0])).toEqual(
      fieldsOf("AgentCapabilityView"),
    );
    expect(keysOf((await client.listPersonalizationCandidates())[0])).toEqual(
      fieldsOf("MemoryCandidateView"),
    );
    expect(keysOf(await client.reconcilePersonalizationMemories())).toEqual(
      fieldsOf("MaintenanceResultView"),
    );
  });

  it("returns the same paged and detailed shapes the native views serialize", async () => {
    const client = webPersonalizationClient;
    const page = await client.queryPersonalizationMemories({});

    expect(keysOf(page)).toEqual(fieldsOf("MemoryPageView"));
    expect(keysOf(page.items[0])).toEqual(fieldsOf("MemorySummaryView"));

    const detail = await client.getPersonalizationMemory(page.items[0]?.id ?? "");
    expect(keysOf(detail)).toEqual(fieldsOf("MemoryDetailView"));
  });

  it("returns the same preview shape the native view serializes, nested types included", async () => {
    const preview = await webPersonalizationClient.previewEffectivePersonalization({
      agentId: "onepiece",
      sessionId: "session-1",
      workspaceKey: "ws-vanehub",
    });

    expect(keysOf(preview)).toEqual(fieldsOf("EffectivePreviewView"));
    expect(keysOf(preview.includedInstructions[0])).toEqual(fieldsOf("PreviewSegmentView"));
    expect(keysOf(preview.memoryExclusions[0])).toEqual(fieldsOf("MemoryExclusionView"));

    const capped = await webPersonalizationClient.previewEffectivePersonalization({
      agentId: "gemini-cli",
      sessionId: "session-1",
    });
    expect(keysOf(capped.excludedInstructions[0])).toEqual(fieldsOf("ExcludedSegmentView"));
  });

  it("returns the same reset and review shapes the native views serialize", async () => {
    const client = webPersonalizationClient;
    const preview = await client.previewPersonalizationReset({ includeArchived: false });

    expect(keysOf(preview)).toEqual(fieldsOf("ResetPreviewView"));

    const outcome = await client.reviewPersonalizationCandidate({
      candidateId: (await client.listPersonalizationCandidates())[0]?.id ?? "",
      action: "reject",
    });
    expect(keysOf(outcome)).toEqual(fieldsOf("ReviewOutcomeView"));
  });

  it("keeps both adapters on the same service boundary", () => {
    // Both are typed as `PersonalizationService`, so a missing method is a compile error -- but an
    // extra one on either side is not, and that is how the two drift apart.
    expect(Object.keys(tauriPersonalizationClient).sort()).toEqual(
      Object.keys(webPersonalizationClient).sort(),
    );
  });
});
