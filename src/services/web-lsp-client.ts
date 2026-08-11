import type { AgentService } from "./agent-service";
import {
  normalizeLspConfiguration,
  normalizeLspServerDiscoveries,
  normalizeLspServerStatuses,
  normalizeLspServerTestResult,
  normalizeLspWorkspaceTrust,
  normalizeLspWorkspaceTrustList,
  normalizeLspWorkspaceTrustUpdate,
} from "./lsp-contract";
import {
  lspLanguageIds,
  lspServerTestPhases,
  type LspConfiguration,
  type LspLanguageId,
  type LspNegotiatedCapabilities,
  type LspProcessState,
  type LspServerDiscovery,
  type LspServerKind,
  type LspServerStatus,
  type LspToolName,
  type LspToolResult,
  type LspToolResultMetadata,
  type LspWorkspaceTrust,
} from "../types/lsp";

type WebLspClient = Pick<AgentService,
  "getLspConfiguration"
  | "saveLspConfiguration"
  | "listLspWorkspaceTrust"
  | "updateLspWorkspaceTrust"
  | "discoverLspServers"
  | "testLspServer"
  | "getLspServerStatus"
>;

const maximumTrustRecords = 128;
const readyTimestamp = "2026-01-01T00:00:00Z";
const virtualExecutables: Record<LspLanguageId, string> = {
  rust: "/mock/lsp/rust-analyzer",
  typescript_javascript: "/mock/lsp/typescript-language-server",
};

const capabilities: LspNegotiatedCapabilities = {
  positionEncoding: "utf16",
  documentSync: "incremental",
  definition: true,
  references: true,
  hover: true,
  diagnostics: true,
};

function defaultConfiguration(): LspConfiguration {
  return {
    enabled: false,
    languages: lspLanguageIds.map((language) => ({
      language,
      enabled: false,
      executableOverride: null,
      initializationOptions: {},
    })),
  };
}

let configuration = defaultConfiguration();
const trustRecords = new Map<string, LspWorkspaceTrust>();
const lifecyclePolls = new Map<LspLanguageId, number>();

function clone<T>(value: T): T {
  return structuredClone(value);
}

function normalizeRoot(root: string): string {
  const normalized = root.trim().replace(/\\/g, "/");
  if (normalized === "/" || /^[A-Za-z]:\/$/.test(normalized)) return normalized;
  return normalized.replace(/\/+$/, "");
}

function serverFor(language: LspLanguageId): LspServerKind {
  return language === "rust" ? "rust_analyzer" : "typescript_language_server";
}

function argumentsFor(language: LspLanguageId): string[] {
  return language === "typescript_javascript" ? ["--stdio"] : [];
}

function discoveryFor(language: LspLanguageId): LspServerDiscovery {
  const languageConfiguration = configuration.languages.find(
    (entry) => entry.language === language,
  );
  return {
    language,
    server: serverFor(language),
    availability: "available",
    executablePath: languageConfiguration?.executableOverride ?? virtualExecutables[language],
    arguments: argumentsFor(language),
    reasonCode: null,
  };
}

function lifecycleState(polls: number): LspProcessState {
  if (polls === 0) return "starting";
  if (polls === 1) return "initializing";
  return "ready";
}

function compareRoots(left: LspWorkspaceTrust, right: LspWorkspaceTrust): number {
  if (left.canonicalRoot < right.canonicalRoot) return -1;
  if (left.canonicalRoot > right.canonicalRoot) return 1;
  return 0;
}

function enabledLanguages(): LspLanguageId[] {
  if (!configuration.enabled || ![...trustRecords.values()].some((entry) => entry.trusted)) {
    return [];
  }
  return configuration.languages
    .filter((entry) => entry.enabled)
    .map((entry) => entry.language);
}

function statusFor(language: LspLanguageId): LspServerStatus {
  const polls = lifecyclePolls.get(language) ?? 0;
  const state = lifecycleState(polls);
  lifecyclePolls.set(language, polls + 1);
  return {
    language,
    server: serverFor(language),
    relativeProjectRoot: ".",
    state,
    restartCount: 0,
    lastResponseAt: state === "ready" ? readyTimestamp : null,
    diagnosticCount: 0,
    reasonCode: null,
    negotiatedCapabilities: state === "ready" ? clone(capabilities) : null,
  };
}

export function resetWebLspMockStateForTest(): void {
  configuration = defaultConfiguration();
  trustRecords.clear();
  lifecyclePolls.clear();
}

export const webLspClient: WebLspClient = {
  async getLspConfiguration() {
    return clone(configuration);
  },

  async saveLspConfiguration(nextConfiguration) {
    configuration = clone(normalizeLspConfiguration(nextConfiguration));
    lifecyclePolls.clear();
  },

  async listLspWorkspaceTrust() {
    const records = [...trustRecords.values()]
      .sort(compareRoots);
    return clone(normalizeLspWorkspaceTrustList(records));
  },

  async updateLspWorkspaceTrust(nextTrust) {
    const update = normalizeLspWorkspaceTrustUpdate(nextTrust);
    const canonicalRoot = normalizeRoot(update.canonicalRoot);
    const existing = trustRecords.get(canonicalRoot);
    if (!existing && trustRecords.size >= maximumTrustRecords) {
      throw new Error(`The Web LSP trust limit of ${maximumTrustRecords} records was reached.`);
    }
    const record = normalizeLspWorkspaceTrust({
      canonicalRoot,
      trusted: update.trusted,
      revision: (existing?.revision ?? 0) + 1,
    });
    trustRecords.set(canonicalRoot, record);
    lifecyclePolls.clear();
    return clone(record);
  },

  async discoverLspServers() {
    return clone(normalizeLspServerDiscoveries(lspLanguageIds.map(discoveryFor)));
  },

  async testLspServer(language) {
    return clone(normalizeLspServerTestResult(
      {
        server: serverFor(language),
        phases: lspServerTestPhases.map((phase) => ({
          phase,
          status: "succeeded",
          reasonCode: null,
        })),
        negotiatedCapabilities: capabilities,
      },
      language,
    ));
  },

  async getLspServerStatus() {
    return clone(normalizeLspServerStatuses(enabledLanguages().map(statusFor)));
  },
};

export interface WebLspToolClient {
  execute(tool: LspToolName): Promise<LspToolResult>;
}

function unavailableToolMetadata(): LspToolResultMetadata {
  return {
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
  };
}

function unavailableToolResult(tool: LspToolName): LspToolResult {
  const metadata = unavailableToolMetadata();
  switch (tool) {
    case "find_definition":
      return { metadata, definitions: [] };
    case "find_references":
      return { metadata, references: [] };
    case "get_hover":
      return { metadata, hover: null };
    case "get_diagnostics":
      return { metadata, diagnostics: [] };
  }
}

export const webLspToolClient: WebLspToolClient = {
  async execute(tool) {
    return clone(unavailableToolResult(tool));
  },
};
