import type {
  LspConfiguration,
  LspLanguageId,
  LspServerDiscovery,
  LspServerStatus,
  LspServerTestResult,
  LspWorkspaceTrust,
  LspWorkspaceTrustUpdate,
} from "../types/lsp";

export interface LspService {
  getLspConfiguration(): Promise<LspConfiguration>;
  saveLspConfiguration(configuration: LspConfiguration): Promise<void>;
  listLspWorkspaceTrust(): Promise<LspWorkspaceTrust[]>;
  updateLspWorkspaceTrust(update: LspWorkspaceTrustUpdate): Promise<LspWorkspaceTrust>;
  discoverLspServers(): Promise<LspServerDiscovery[]>;
  testLspServer(language: LspLanguageId): Promise<LspServerTestResult>;
  // Managed installation of a declared distribution. Only languages the backend describes with one
  // can be installed; the Web adapter rejects rather than reporting a download it cannot perform.
  installLspServer(language: LspLanguageId): Promise<void>;
  uninstallLspServer(language: LspLanguageId): Promise<void>;
  getLspServerStatus(): Promise<LspServerStatus[]>;
}
