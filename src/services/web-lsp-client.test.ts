import { beforeEach, describe, expect, it } from "vitest";
import type { LspConfiguration } from "../types/lsp";
import { webAgentClient } from "./web-agent-client";
import { resetWebLspMockStateForTest, webLspToolClient } from "./web-lsp-client";

const enabledRustConfiguration: LspConfiguration = {
  enabled: true,
  languages: [
    {
      language: "rust",
      enabled: true,
      executableOverride: "C:/tools/rust-analyzer.exe",
      initializationOptions: { diagnostics: { enable: true } },
    },
    {
      language: "typescript_javascript",
      enabled: false,
      executableOverride: null,
      initializationOptions: {},
    },
  ],
};

describe("Web LSP adapter", () => {
  beforeEach(() => resetWebLspMockStateForTest());

  it("starts disabled and persists defensive configuration copies", async () => {
    const initial = await webAgentClient.getLspConfiguration();
    expect(initial.enabled).toBe(false);
    expect(initial.languages.every((entry) => !entry.enabled)).toBe(true);

    await webAgentClient.saveLspConfiguration(enabledRustConfiguration);
    const saved = await webAgentClient.getLspConfiguration();
    expect(saved).toEqual(enabledRustConfiguration);
    saved.languages[0].initializationOptions.diagnostics = false;
    await expect(webAgentClient.getLspConfiguration())
      .resolves.toEqual(enabledRustConfiguration);
  });

  it("normalizes trust roots and advances revisions monotonically", async () => {
    await expect(webAgentClient.updateLspWorkspaceTrust({
      canonicalRoot: "D:\\code\\app\\",
      trusted: true,
    })).resolves.toEqual({ canonicalRoot: "D:/code/app", trusted: true, revision: 1 });
    await expect(webAgentClient.updateLspWorkspaceTrust({
      canonicalRoot: "D:/code/app",
      trusted: false,
    })).resolves.toEqual({ canonicalRoot: "D:/code/app", trusted: false, revision: 2 });

    await webAgentClient.updateLspWorkspaceTrust({
      canonicalRoot: "C:/code/app",
      trusted: true,
    });
    await expect(webAgentClient.listLspWorkspaceTrust()).resolves.toEqual([
      { canonicalRoot: "C:/code/app", trusted: true, revision: 1 },
      { canonicalRoot: "D:/code/app", trusted: false, revision: 2 },
    ]);
  });

  it("bounds the in-memory trust registry", async () => {
    for (let index = 0; index < 128; index += 1) {
      await webAgentClient.updateLspWorkspaceTrust({
        canonicalRoot: `C:/workspace/${index}`,
        trusted: true,
      });
    }
    await expect(webAgentClient.updateLspWorkspaceTrust({
      canonicalRoot: "C:/workspace/overflow",
      trusted: true,
    })).rejects.toThrow("trust limit");
    await expect(webAgentClient.listLspWorkspaceTrust()).resolves.toHaveLength(128);
  });

  it("returns deterministic discovery and isolated server-test results", async () => {
    const firstDiscovery = await webAgentClient.discoverLspServers();
    expect(await webAgentClient.discoverLspServers()).toEqual(firstDiscovery);
    expect(firstDiscovery).toEqual([
      {
        language: "rust",
        server: "rust_analyzer",
        availability: "available",
        executablePath: "/mock/lsp/rust-analyzer",
        arguments: [],
        reasonCode: null,
      },
      {
        language: "typescript_javascript",
        server: "typescript_language_server",
        availability: "available",
        executablePath: "/mock/lsp/typescript-language-server",
        arguments: ["--stdio"],
        reasonCode: null,
      },
    ]);

    const result = await webAgentClient.testLspServer("rust");
    expect(result.phases.every((phase) => phase.status === "succeeded")).toBe(true);
    await expect(webAgentClient.testLspServer("rust")).resolves.toEqual(result);
    await expect(webAgentClient.getLspServerStatus()).resolves.toEqual([]);
  });

  it("advances configured trusted servers through deterministic lifecycle states", async () => {
    await webAgentClient.saveLspConfiguration(enabledRustConfiguration);
    await webAgentClient.updateLspWorkspaceTrust({
      canonicalRoot: "D:/code/app",
      trusted: true,
    });

    const starting = await webAgentClient.getLspServerStatus();
    const initializing = await webAgentClient.getLspServerStatus();
    const ready = await webAgentClient.getLspServerStatus();
    expect(starting.map((status) => status.state)).toEqual(["starting"]);
    expect(initializing.map((status) => status.state)).toEqual(["initializing"]);
    expect(ready.map((status) => status.state)).toEqual(["ready"]);
    expect(ready[0].negotiatedCapabilities).not.toBeNull();
    await expect(webAgentClient.getLspServerStatus()).resolves.toEqual(ready);

    await webAgentClient.updateLspWorkspaceTrust({
      canonicalRoot: "D:/code/app",
      trusted: false,
    });
    await expect(webAgentClient.getLspServerStatus()).resolves.toEqual([]);
  });

  it.each([
    ["find_definition", "definitions"],
    ["find_references", "references"],
    ["get_hover", "hover"],
    ["get_diagnostics", "diagnostics"],
  ] as const)("returns a deterministic unavailable %s tool result", async (tool, payloadKey) => {
    const first = await webLspToolClient.execute(tool);
    const second = await webLspToolClient.execute(tool);

    expect(second).toEqual(first);
    expect(first.metadata).toEqual({
      status: "unavailable",
      server: null,
      language: null,
      document_version: null,
      stale: false,
      returned_count: 0,
      total: 0,
      truncated: false,
      filtered_count: 0,
      reason_code: "web_runtime_unavailable",
    });
    expect(Object.keys(first).sort()).toEqual(["metadata", payloadKey].sort());
    expect(first).toHaveProperty(payloadKey, payloadKey === "hover" ? null : []);
  });
});
