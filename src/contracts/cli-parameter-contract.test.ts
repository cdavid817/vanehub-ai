import { execFileSync } from "node:child_process";
import { readFileSync } from "node:fs";
import { describe, expect, it } from "vitest";
import {
  cliParameterCatalogVersion,
  cliParameterDefinitions,
  defaultCliParameterSelections,
  editableCliParameterDefinitions,
} from "../services/cli-parameter-registry";
import {
  cliArgumentSegmentValues,
  renderCliParameterSegments,
  tomlBasicString,
} from "../services/cli-parameter-renderer";
import { webCliParameterClient } from "../services/web-cli-parameter-client";
import { managedCliAgentIds } from "../types/agent";

const nativeCatalog: unknown = JSON.parse(
  readFileSync(
    "src-tauri/src/contexts/tooling/cli_parameters/catalog/catalog.v2.json",
    "utf8",
  ),
);

interface NativeParameter {
  id: string;
  /** Absent in the canonical file means `user-editable`; serde supplies that default on load. */
  ownership?: string;
  renderer: { kind: string; slot: string };
  launchScopes: string[];
}

function nativeOwnership(parameter: NativeParameter): string {
  return parameter.ownership ?? "user-editable";
}

function nativeParameters(agentId: string): NativeParameter[] {
  const catalog = nativeCatalog as { agents: { agentId: string; parameters: NativeParameter[] }[] };
  return catalog.agents.find((agent) => agent.agentId === agentId)?.parameters ?? [];
}

describe("CLI parameter contract", () => {
  it("regenerates the frontend catalog byte-for-byte from the canonical registry", () => {
    const before = readFileSync("src/generated/cli-parameter-catalog.json", "utf8");

    execFileSync("node", ["scripts/generate-cli-parameter-catalog.mjs", "--check"], {
      stdio: "pipe",
    });

    // A second generation must also be stable: a non-deterministic generator would pass `--check`
    // once and then produce a different file on the next developer's machine.
    execFileSync("node", ["scripts/generate-cli-parameter-catalog.mjs"], { stdio: "pipe" });
    expect(readFileSync("src/generated/cli-parameter-catalog.json", "utf8")).toBe(before);
  });

  it("carries every canonical parameter across to the frontend registry, in order", () => {
    for (const agentId of managedCliAgentIds) {
      expect(cliParameterDefinitions(agentId).map((definition) => definition.id)).toEqual(
        nativeParameters(agentId).map((parameter) => parameter.id),
      );
    }
  });

  it("agrees with the canonical registry about ownership, slot, and launch scope", () => {
    for (const agentId of managedCliAgentIds) {
      const native = new Map(nativeParameters(agentId).map((entry) => [entry.id, entry]));
      for (const definition of cliParameterDefinitions(agentId)) {
        const source = native.get(definition.id);
        expect(source).toBeDefined();
        expect(definition.ownership).toBe(nativeOwnership(source!));
        expect(definition.renderer.slot).toBe(source!.renderer.slot);
        expect(definition.renderer.kind).toBe(source!.renderer.kind);
        expect(definition.launchScopes).toEqual(source!.launchScopes);
      }
    }
  });

  it("keeps policy-governed and runtime-reserved parameters out of the editable set", () => {
    const governed = managedCliAgentIds.flatMap((agentId) =>
      nativeParameters(agentId)
        .filter((parameter) => nativeOwnership(parameter) !== "user-editable")
        .map((parameter) => `${agentId}:${parameter.id}`),
    );
    expect(governed.length).toBeGreaterThan(0);

    const editable = new Set(
      managedCliAgentIds.flatMap((agentId) =>
        editableCliParameterDefinitions(agentId).map((definition) => `${agentId}:${definition.id}`),
      ),
    );
    for (const id of governed) expect(editable.has(id)).toBe(false);
  });

  it("never emits an approval, permission, sandbox, or bypass flag from the editable set", () => {
    const forbidden = [
      "--permission-mode",
      "--dangerously-skip-permissions",
      "--dangerously-bypass-approvals-and-sandbox",
      "--yolo",
      "--sandbox",
      "--approval-mode",
      "--ask-for-approval",
      "--full-auto",
    ];
    for (const agentId of managedCliAgentIds) {
      const flags = editableCliParameterDefinitions(agentId).map((definition) =>
        definition.renderer.kind === "positive-negative-flag"
          ? definition.renderer.positiveFlag
          : definition.renderer.flag,
      );
      for (const flag of forbidden) expect(flags).not.toContain(flag);
    }
  });

  it("renders each strategy the way the canonical encoder does", () => {
    const definitions = cliParameterDefinitions("codex-cli");
    const model = definitions.find((entry) => entry.id === "model")!;
    const ephemeral = definitions.find((entry) => entry.id === "ephemeral")!;

    expect(
      cliArgumentSegmentValues(
        renderCliParameterSegments(
          [model],
          { model: { state: "value", value: "gpt-5.5" } },
          "chat",
        ),
      ),
    ).toEqual(["--model", "gpt-5.5"]);

    expect(
      cliArgumentSegmentValues(
        renderCliParameterSegments([ephemeral], { ephemeral: { state: "value", value: true } }, "chat"),
      ),
    ).toEqual(["--ephemeral"]);

    // Inheritance renders nothing at all — not the provider's default spelled out.
    expect(
      cliArgumentSegmentValues(
        renderCliParameterSegments([model], { model: { state: "inherit" } }, "chat"),
      ),
    ).toEqual([]);
  });

  it("encodes a TOML basic string the way the canonical encoder does", () => {
    expect(tomlBasicString('a"b\\c')).toBe('"a\\"b\\\\c"');
    expect(tomlBasicString("line\nbreak")).toBe('"line\\nbreak"');
    expect(tomlBasicString("bell\u0007")).toBe('"bell\\u0007"');
  });

  it("keeps a whitespace-bearing value as one argv token", () => {
    const includeDirectories = cliParameterDefinitions("gemini-cli").find(
      (entry) => entry.id === "includeDirectories",
    )!;

    const segments = renderCliParameterSegments(
      [includeDirectories],
      { includeDirectories: { state: "value", value: ["C:/Program Files/app"] } },
      "chat",
    );

    expect(cliArgumentSegmentValues(segments)).toContain("C:/Program Files/app");
  });

  it("previews a draft through the Web adapter without persisting it", async () => {
    const before = await webCliParameterClient.listCliParameterProfiles();
    const claude = before.find((profile) => profile.agentId === "claude-code")!;

    const preview = await webCliParameterClient.previewCliParameterProfile({
      agentId: "claude-code",
      catalogVersion: cliParameterCatalogVersion,
      scope: "chat",
      selections: { ...defaultCliParameterSelections("claude-code"), model: { state: "value", value: "opus" } },
    });

    expect(preview.segments.global.map((token) => token.value)).toEqual(["--model", "opus"]);
    expect(preview.requestId).toBeUndefined();
    const after = await webCliParameterClient.listCliParameterProfiles();
    expect(after.find((profile) => profile.agentId === "claude-code")!.revision).toBe(claude.revision);
  });

  it("scopes an interactive-only parameter out of the chat preview", () => {
    const definitions = editableCliParameterDefinitions("codex-cli");
    const interactiveOnly = definitions.find(
      (entry) => entry.launchScopes.length === 1 && entry.launchScopes[0] === "interactive",
    );
    expect(interactiveOnly).toBeDefined();

    const selections = { [interactiveOnly!.id]: { state: "value", value: true } } as const;
    expect(
      cliArgumentSegmentValues(renderCliParameterSegments(definitions, selections, "chat")),
    ).toEqual([]);
    expect(
      cliArgumentSegmentValues(renderCliParameterSegments(definitions, selections, "interactive")),
    ).not.toEqual([]);
  });
});
