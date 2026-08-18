import { nowIso } from "./web-mock-clock";
import type { RetrievalConfiguration, RetrievalIndexStatus, Session } from "../types/agent";
import type {
  CodeIndexAuditEntry,
  CodeIndexPhase,
  CodeIndexStatus,
  CodeIndexWorkspace,
} from "../types/code-index";
import { codeIndexLanguages } from "../types/code-index";

/** Mock retrieval configuration (`add-retrieval-vector-search`) — a global singleton mirroring
 * the real `retrieval_configuration` table's single row; starts unconfigured like a fresh
 * install (design doc §7.4). */
let webRetrievalConfiguration: RetrievalConfiguration = {
  sourceProfileId: null,
  embeddingModel: null,
  automaticCodeIndexMode: "disabled",
};

/** Mock retrieval index status — a single global aggregate, mirroring the real one across every
 * agent and every `scope_folder` (design doc §7.4). Seeded with plausible, self-consistent counts
 * so a settings UI has something realistic to render before ever calling `rebuildRetrievalIndex`.
 * `lastFailureCategory` is deliberately always `null`: the Web/mock runtime guarantees the same
 * contract shape and observable behavior as the real one, not algorithmic equivalence with the
 * Rust-side failure classification (design doc §7.5). */
const seededWebRetrievalIndexStatus = (): RetrievalIndexStatus => ({ indexed: 12, pending: 3, failed: 2, lastFailureCategory: null });
let webRetrievalIndexStatus: RetrievalIndexStatus = seededWebRetrievalIndexStatus();

let nextWebCodeIndexId = 1;
let nextWebCodeAuditId = 1;
export const webCodeIndexes = new Map<string, CodeIndexWorkspace>();
let webCodeIndexAudit: CodeIndexAuditEntry[] = [];

export function readWebRetrievalConfiguration(): RetrievalConfiguration {
  return webRetrievalConfiguration;
}

export function writeWebRetrievalConfiguration(value: RetrievalConfiguration) {
  webRetrievalConfiguration = value;
}

export function readWebRetrievalIndexStatus(): RetrievalIndexStatus {
  return webRetrievalIndexStatus;
}

export function readWebCodeIndexAudit(): CodeIndexAuditEntry[] {
  return webCodeIndexAudit;
}

export function writeWebCodeIndexAudit(value: CodeIndexAuditEntry[]) {
  webCodeIndexAudit = value;
}

export function takeNextWebCodeIndexId() {
  const id = nextWebCodeIndexId;
  nextWebCodeIndexId += 1;
  return id;
}

export function emptyCodeIndexStatus(phase: CodeIndexPhase): CodeIndexStatus {
  return {
    phase,
    totalFiles: 0,
    processedFiles: 0,
    failedFiles: 0,
    totalChunks: 0,
    processedChunks: 0,
    pendingChunks: 0,
    indexedChunks: 0,
    failedChunks: 0,
    redactionCount: 0,
    estimatedEmbeddingRequests: 0,
    lastFailureCategory: null,
    updatedAt: nowIso(),
  };
}

export function cloneCodeIndex(workspace: CodeIndexWorkspace): CodeIndexWorkspace {
  return structuredClone(workspace);
}

export function requireWebCodeIndex(workspaceId: string): CodeIndexWorkspace {
  const workspace = webCodeIndexes.get(workspaceId);
  if (!workspace) throw new Error("Code index workspace was not found.");
  return workspace;
}

export function updateWebCodeIndexPhase(workspace: CodeIndexWorkspace, phase: CodeIndexPhase) {
  const status = workspace.status;
  if (phase === "parsing") {
    Object.assign(status, {
      totalFiles: 18, processedFiles: 6, failedFiles: 0,
      totalChunks: 24, processedChunks: 0, pendingChunks: 24,
      indexedChunks: 0, failedChunks: 0, redactionCount: 2,
      estimatedEmbeddingRequests: 1,
    });
  } else if (phase === "awaiting_embedding_confirmation") {
    Object.assign(status, {
      totalFiles: 18, processedFiles: 18, failedFiles: 0,
      totalChunks: 54, processedChunks: 0, pendingChunks: 54,
      indexedChunks: 0, failedChunks: 0, redactionCount: 4,
      estimatedEmbeddingRequests: 2,
    });
  } else if (phase === "ready") {
    status.processedChunks = status.totalChunks;
    status.pendingChunks = 0;
    status.indexedChunks = status.totalChunks;
    status.failedChunks = 0;
  }
  status.phase = phase;
  status.updatedAt = nowIso();
}

export function recordWebCodeIndexAudit(workspaceId: string, event: CodeIndexAuditEntry["event"]) {
  webCodeIndexAudit = [{
    auditId: nextWebCodeAuditId,
    workspaceId,
    relativePath: null,
    event,
    reason: null,
    itemCount: 1,
    createdAt: nowIso(),
  }, ...webCodeIndexAudit].slice(0, 200);
  nextWebCodeAuditId += 1;
}

/** Called by the composition root's `createSession`, which cannot own this state: the automatic
 * discovery path reads the retrieval configuration and mints a workspace id. */
export function discoverWebSessionCodeIndex(session: Session) {
  const mode = webRetrievalConfiguration.automaticCodeIndexMode;
  const root = session.worktreePath ?? session.folder ?? session.projectPath;
  if (session.agentId !== "onepiece" || session.remoteWorkspace || !root || mode === "disabled") {
    return;
  }
  const normalizedRoot = root.replaceAll("\\", "/").replace(/\/$/, "").toLocaleLowerCase();
  const existing = [...webCodeIndexes.values()].find((workspace) => (
    workspace.canonicalRoot.replaceAll("\\", "/").replace(/\/$/, "").toLocaleLowerCase()
      === normalizedRoot
  ));
  if (existing) return;
  const displayName = root.split(/[\\/]/).filter(Boolean).at(-1) ?? root;
  const workspace: CodeIndexWorkspace = {
    workspaceId: `web-code-index-${nextWebCodeIndexId}`,
    canonicalRoot: root,
    displayName,
    origin: "automatic",
    enabled: true,
    mode,
    selectedRoots: [""],
    languages: [...codeIndexLanguages],
    exclusionPatterns: [],
    maxFileBytes: 100 * 1024,
    indexVersion: "1",
    generation: 1,
    status: emptyCodeIndexStatus("scanning"),
  };
  nextWebCodeIndexId += 1;
  webCodeIndexes.set(workspace.workspaceId, workspace);
}

export function resetWebRetrievalForTest() {
  webRetrievalConfiguration = {
    sourceProfileId: null,
    embeddingModel: null,
    automaticCodeIndexMode: "disabled",
  };
  webRetrievalIndexStatus = seededWebRetrievalIndexStatus();
  nextWebCodeIndexId = 1;
  nextWebCodeAuditId = 1;
  webCodeIndexes.clear();
  webCodeIndexAudit = [];
}

export function searchWebCodeIndex(workspaceId: string, query: string) {
  const workspace = requireWebCodeIndex(workspaceId);
  const normalizedQuery = query.trim().toLocaleLowerCase();
  const includesSourceRoot = workspace.selectedRoots.some((root) => root === "" || root === "src");
  const sourceExcluded = workspace.exclusionPatterns.some((pattern) => pattern === "src/**" || pattern === "**/*.ts");
  if (workspace.status.phase !== "ready" || !workspace.languages.includes("typescript")
    || !includesSourceRoot || sourceExcluded || normalizedQuery !== "handle_login") return [];
  if (workspace.canonicalRoot.toLocaleLowerCase().includes("second")) return [];
  return [{
    filePath: "src/auth.ts",
    startLine: 12,
    endLine: 20,
    language: "typescript",
    symbolName: "handle_login",
    symbolKind: "function",
    snippet: "export async function handle_login(request: Request) { /* redacted */ }",
    matchedVia: workspace.mode === "local" ? "keyword" : "hybrid",
  }];
}
