import { beforeEach, describe, expect, it, vi } from "vitest";

const { invokeMock } = vi.hoisted(() => ({ invokeMock: vi.fn() }));

vi.mock("@tauri-apps/api/core", () => ({ invoke: invokeMock }));
vi.mock("@tauri-apps/api/event", () => ({ listen: vi.fn() }));
vi.mock("@tauri-apps/plugin-dialog", () => ({ open: vi.fn() }));
vi.mock("@tauri-apps/plugin-opener", () => ({ openUrl: vi.fn() }));

import { tauriAgentClient } from "./tauri-agent-client";
import type { LspConfiguration } from "../types/lsp";
import { lspTestDescriptors } from "../test/lsp-fixtures";

const configuration: LspConfiguration = {
  enabled: false,
  languages: [
    {
      language: "rust",
      enabled: false,
      executableOverride: null,
      startupArguments: null,
      initializationOptions: {},
    },
    {
      language: "typescript_javascript",
      enabled: false,
      executableOverride: null,
      startupArguments: null,
      initializationOptions: {},
    },
  ],
  descriptors: lspTestDescriptors(),
};

const trust = {
  canonicalRoot: "D:/code/app",
  trusted: true,
  revision: 1,
};

const discoveries = [
  {
    language: "rust",
    server: "rust_analyzer",
    availability: "unavailable",
    executablePath: null,
    arguments: [],
    reasonCode: "executable_not_found",
  },
  {
    language: "typescript_javascript",
    server: "typescript_language_server",
    availability: "unavailable",
    executablePath: null,
    arguments: ["--stdio"],
    reasonCode: "executable_not_found",
  },
];

const serverTest = {
  server: "rust_analyzer",
  phases: [
    { phase: "discovery", status: "failed", reasonCode: "executable_unavailable" },
    { phase: "spawn", status: "skipped", reasonCode: null },
    { phase: "initialize", status: "skipped", reasonCode: null },
    { phase: "cleanup", status: "skipped", reasonCode: null },
  ],
  negotiatedCapabilities: null,
};

describe("Tauri LSP adapter", () => {
  beforeEach(() => {
    invokeMock.mockReset().mockImplementation((command: string) => {
      if (command === "get_lsp_configuration") return Promise.resolve(configuration);
      if (command === "list_lsp_workspace_trust") return Promise.resolve([trust]);
      if (command === "update_lsp_workspace_trust") return Promise.resolve(trust);
      if (command === "discover_lsp_servers") return Promise.resolve(discoveries);
      if (command === "test_lsp_server") return Promise.resolve(serverTest);
      if (command === "list_lsp_server_status") return Promise.resolve([]);
      return Promise.resolve(undefined);
    });
  });

  it("maps every LSP operation to its registered command", async () => {
    await expect(tauriAgentClient.getLspConfiguration()).resolves.toEqual(configuration);
    await tauriAgentClient.saveLspConfiguration(configuration);
    await expect(tauriAgentClient.listLspWorkspaceTrust()).resolves.toEqual([trust]);
    await expect(tauriAgentClient.updateLspWorkspaceTrust({
      canonicalRoot: "D:/code/app",
      trusted: true,
    })).resolves.toEqual(trust);
    await expect(tauriAgentClient.discoverLspServers()).resolves.toEqual(discoveries);
    await expect(tauriAgentClient.testLspServer("rust")).resolves.toEqual(serverTest);
    await expect(tauriAgentClient.getLspServerStatus()).resolves.toEqual([]);

    expect(invokeMock.mock.calls).toEqual([
      ["get_lsp_configuration"],
      ["save_lsp_configuration", { configuration }],
      ["list_lsp_workspace_trust"],
      ["update_lsp_workspace_trust", {
        update: { canonicalRoot: "D:/code/app", trusted: true },
      }],
      ["discover_lsp_servers"],
      ["test_lsp_server", { input: { language: "rust" } }],
      ["list_lsp_server_status"],
    ]);
  });

  it("rejects malformed native payloads before they reach service consumers", async () => {
    // An empty language list used to be the malformed case, because the payload had to name every
    // supported language. It is legal now, so the check that remains is a language id no
    // descriptor in the same payload accounts for.
    invokeMock.mockResolvedValueOnce({
      ...configuration,
      languages: [...configuration.languages, {
        language: "go",
        enabled: true,
        executableOverride: null,
        startupArguments: null,
        initializationOptions: {},
      }],
    });
    await expect(tauriAgentClient.getLspConfiguration())
      .rejects.toThrow("invalid LSP response");

    invokeMock.mockResolvedValueOnce({ ...configuration, descriptors: [{ language: "rust" }] });
    await expect(tauriAgentClient.getLspConfiguration())
      .rejects.toThrow("invalid LSP response");
  });
});
