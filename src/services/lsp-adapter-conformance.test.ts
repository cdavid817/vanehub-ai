import { beforeEach, describe, expect, it, vi } from "vitest";

const { invokeMock } = vi.hoisted(() => ({ invokeMock: vi.fn() }));

vi.mock("@tauri-apps/api/core", () => ({ invoke: invokeMock }));
vi.mock("@tauri-apps/api/event", () => ({ listen: vi.fn() }));
vi.mock("@tauri-apps/plugin-dialog", () => ({ open: vi.fn() }));
vi.mock("@tauri-apps/plugin-opener", () => ({ openUrl: vi.fn() }));

import type { AgentService } from "./agent-service";
import {
  normalizeLspConfiguration,
  normalizeLspServerDiscoveries,
  normalizeLspServerStatuses,
  normalizeLspServerTestResult,
  normalizeLspWorkspaceTrust,
  normalizeLspWorkspaceTrustList,
} from "./lsp-contract";
import { tauriAgentClient } from "./tauri-agent-client";
import { webAgentClient } from "./web-agent-client";
import { resetWebLspMockStateForTest } from "./web-lsp-client";

const lspMethods = [
  "getLspConfiguration",
  "saveLspConfiguration",
  "listLspWorkspaceTrust",
  "updateLspWorkspaceTrust",
  "discoverLspServers",
  "testLspServer",
  "getLspServerStatus",
] as const satisfies readonly (keyof AgentService)[];

type LspAdapter = Pick<AgentService, (typeof lspMethods)[number]>;

const configuration = {
  enabled: false,
  languages: [
    {
      language: "rust",
      enabled: false,
      executableOverride: null,
      initializationOptions: {},
    },
    {
      language: "typescript_javascript",
      enabled: false,
      executableOverride: null,
      initializationOptions: {},
    },
  ],
};

const trust = { canonicalRoot: "D:/code/app", trusted: true, revision: 1 };
const discoveries = [
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
];
const capabilities = {
  positionEncoding: "utf16",
  documentSync: "incremental",
  definition: true,
  references: true,
  hover: true,
  diagnostics: true,
};
const serverTest = {
  server: "rust_analyzer",
  phases: [
    { phase: "discovery", status: "succeeded", reasonCode: null },
    { phase: "spawn", status: "succeeded", reasonCode: null },
    { phase: "initialize", status: "succeeded", reasonCode: null },
    { phase: "cleanup", status: "succeeded", reasonCode: null },
  ],
  negotiatedCapabilities: capabilities,
};

function configureNativeResponses(): void {
  invokeMock.mockImplementation((command: string) => {
    if (command === "get_lsp_configuration") return Promise.resolve(configuration);
    if (command === "list_lsp_workspace_trust") return Promise.resolve([trust]);
    if (command === "update_lsp_workspace_trust") return Promise.resolve(trust);
    if (command === "discover_lsp_servers") return Promise.resolve(discoveries);
    if (command === "test_lsp_server") return Promise.resolve(serverTest);
    if (command === "list_lsp_server_status") return Promise.resolve([]);
    return Promise.resolve(undefined);
  });
}

async function exerciseContract(adapter: LspAdapter): Promise<void> {
  const currentConfiguration = normalizeLspConfiguration(await adapter.getLspConfiguration());
  await adapter.saveLspConfiguration(currentConfiguration);
  normalizeLspWorkspaceTrustList(await adapter.listLspWorkspaceTrust());
  normalizeLspWorkspaceTrust(await adapter.updateLspWorkspaceTrust({
    canonicalRoot: "D:/code/app",
    trusted: true,
  }));
  normalizeLspServerDiscoveries(await adapter.discoverLspServers());
  normalizeLspServerTestResult(await adapter.testLspServer("rust"), "rust");
  normalizeLspServerStatuses(await adapter.getLspServerStatus());
}

describe("LSP adapter conformance", () => {
  beforeEach(() => {
    invokeMock.mockReset();
    configureNativeResponses();
    resetWebLspMockStateForTest();
  });

  it("exposes the same callable LSP service surface in Tauri and Web runtimes", async () => {
    for (const method of lspMethods) {
      expect(typeof tauriAgentClient[method]).toBe("function");
      expect(typeof webAgentClient[method]).toBe("function");
    }
    await expect(exerciseContract(tauriAgentClient)).resolves.toBeUndefined();
    await expect(exerciseContract(webAgentClient)).resolves.toBeUndefined();
  });

  it.each([
    ["configuration", { ...configuration, languages: [] },
      () => tauriAgentClient.getLspConfiguration()],
    ["trust list", [trust, { ...trust, revision: 2 }],
      () => tauriAgentClient.listLspWorkspaceTrust()],
    ["trust update", { ...trust, revision: -1 },
      () => tauriAgentClient.updateLspWorkspaceTrust({
        canonicalRoot: "D:/code/app", trusted: true,
      })],
    ["discovery", [{ ...discoveries[0], server: "typescript_language_server" }, discoveries[1]],
      () => tauriAgentClient.discoverLspServers()],
    ["server identity", { ...serverTest, server: "typescript_language_server" },
      () => tauriAgentClient.testLspServer("rust")],
    ["test phase", {
      ...serverTest,
      phases: serverTest.phases.map((phase) => (
        phase.phase === "spawn" ? { ...phase, reasonCode: "spawn_failed" } : phase
      )),
    }, () => tauriAgentClient.testLspServer("rust")],
    ["status", [{
      language: "rust",
      server: "rust_analyzer",
      relativeProjectRoot: ".",
      state: "ready",
      restartCount: 0,
      lastResponseAt: "not-a-time",
      diagnosticCount: 0,
      reasonCode: null,
      negotiatedCapabilities: capabilities,
    }], () => tauriAgentClient.getLspServerStatus()],
  ])("rejects an invalid native %s payload", async (_label, payload, operation) => {
    invokeMock.mockResolvedValueOnce(payload);
    await expect(operation()).rejects.toThrow("invalid LSP response");
  });
});
