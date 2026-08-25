import type { AgentService } from "./agent-service";
import { rankFileCandidates } from "./file-search-ranking";
import {
  availableContext,
  diffFixture,
  directoryFixtures,
  documentFixtures,
  fileFixtures,
  inspectionCapabilitiesFixture,
  logFixtures,
  searchFixtures,
  statusFixture,
} from "./web-session-workspace-fixtures";
import { sessionWorkspaceLimits } from "../session-workspace/session-workspace-limits";
import type {
  FileContent,
  SessionLogCoverage,
  SessionLogCoverageState,
  SessionLogEntry,
  SessionLogQuery,
} from "../types/session-workspace";
import type { FolderOpenerAvailability, FolderOpenerId, FolderOpenerPreferences } from "../types/folder-opener";

type SessionWorkspaceMethods = Pick<
  AgentService,
  | "getWorkspaceInspectionCapabilities"
  | "listSessionDirectory"
  | "readSessionFile"
  | "listSessionDocuments"
  | "searchSessionFiles"
  | "getSessionGitStatus"
  | "getSessionGitDiff"
  | "listSessionLogs"
  | "getSessionLogRecord"
  | "exportSessionLogs"
  | "listFolderOpeners"
  | "refreshFolderOpeners"
  | "getFolderOpenerPreferences"
  | "saveFolderOpenerPreferences"
  | "openSessionFolder"
  | "subscribeFolderOpenerEvents"
>;


const mockOpeners: FolderOpenerAvailability[] = [
  ["vscode", "editor", true],
  ["file-explorer", "file-manager", true],
  ["windows-terminal", "terminal", true],
  ["git-bash", "terminal", true],
  ["intellij-idea", "ide", false],
  ["webstorm", "ide", false],
].map(([id, category, available]) => ({
  id: id as FolderOpenerId,
  category: category as FolderOpenerAvailability["category"],
  status: available ? "available" : "not-installed",
  executablePath: available ? `[WEB MOCK] ${id}` : null,
  version: available ? "mock" : null,
  edition: null,
  detectionSource: "web-mock",
  iconKey: id as FolderOpenerId,
  reason: available ? null : "not-installed",
}));
let mockPreferences: FolderOpenerPreferences = {
  configuredDefaultOpenerId: "vscode",
  effectiveDefaultOpenerId: "vscode",
  enabledOpenerIds: ["vscode", "file-explorer", "windows-terminal", "git-bash"],
  fallbackActive: false,
};
const openerSubscribers = new Set<() => void>();

export const webSessionWorkspaceClient: SessionWorkspaceMethods = {
  async listFolderOpeners() { return mockOpeners.map((item) => ({ ...item })); },
  async refreshFolderOpeners() { return mockOpeners.map((item) => ({ ...item })); },
  async getFolderOpenerPreferences() { return { ...mockPreferences, enabledOpenerIds: [...mockPreferences.enabledOpenerIds] }; },
  async saveFolderOpenerPreferences(input) {
    const enabled = [...new Set(input.enabledOpenerIds)];
    if (!enabled.includes("file-explorer")) throw new Error("File Explorer must remain enabled.");
    if (!enabled.includes(input.configuredDefaultOpenerId)) throw new Error("Default folder opener must be enabled.");
    if (!mockOpeners.some((item) => item.id === input.configuredDefaultOpenerId && item.status === "available")) throw new Error("Default folder opener must be available.");
    mockPreferences = { configuredDefaultOpenerId: input.configuredDefaultOpenerId, effectiveDefaultOpenerId: input.configuredDefaultOpenerId, enabledOpenerIds: enabled, fallbackActive: false };
    openerSubscribers.forEach((handler) => handler());
    return { ...mockPreferences, enabledOpenerIds: [...enabled] };
  },
  async openSessionFolder(_sessionId, openerId) { return { status: "unavailable", openerId, reason: "web-runtime" }; },
  async subscribeFolderOpenerEvents(handler) { openerSubscribers.add(handler); return () => openerSubscribers.delete(handler); },
  async listSessionDirectory(_sessionId, path = "") {
    return {
      context: availableContext,
      path,
      items: directoryFixtures[path] ?? [],
      truncated: false,
      nextCursor: null,
    };
  },
  async readSessionFile(_sessionId, path): Promise<FileContent> {
    const content = fileFixtures[path];
    if (content === undefined) return { path, name: path.split("/").pop() ?? path, status: "missing", size: 0, content: null };
    return { path, name: path.split("/").pop() ?? path, status: "text", size: content.length, content };
  },
  async listSessionDocuments() {
    return { context: availableContext, items: documentFixtures, truncated: false, nextCursor: null };
  },
  async searchSessionFiles(_sessionId, query, maxResults = 8) {
    const items = rankFileCandidates(query, searchFixtures, maxResults);
    return { context: availableContext, items, truncated: items.length < searchFixtures.length };
  },
  /**
   * The browser build inspects a fixture, and says so.
   *
   * `simulated` is its own provider rather than `local`: a demo that claimed to be reading this
   * machine would send somebody looking for files that are not there. Everything it can answer is
   * available, because the fixture really does contain all of it - the honest gap is the provider
   * name, not a pretend missing prerequisite.
   */
  async getWorkspaceInspectionCapabilities() {
    return structuredClone(inspectionCapabilitiesFixture);
  },
  async getSessionGitStatus() {
    return statusFixture;
  },
  async getSessionGitDiff(_sessionId, _path, source) {
    return { ...diffFixture, source };
  },
  async listSessionLogs(input) {
    const normalizedSearch = input.search.trim().toLocaleLowerCase();
    const filtered = logFixtures.filter((entry) => {
      if (input.levels.length > 0 && !input.levels.includes(entry.level)) return false;
      if (!matchesMockCorrelation(entry, input)) return false;
      if (!normalizedSearch) return true;
      return `${entry.category} ${entry.message} ${JSON.stringify(entry.context)}`.toLocaleLowerCase().includes(normalizedSearch);
    });
    const offset = Number.parseInt(input.cursor ?? "0", 10) || 0;
    const limit = Math.min(input.limit ?? sessionWorkspaceLimits.logPage, sessionWorkspaceLimits.logPage);
    const items = filtered.slice(offset, offset + limit);
    const nextOffset = offset + items.length;
    return {
      items,
      truncated: nextOffset < filtered.length,
      nextCursor: nextOffset < filtered.length ? String(nextOffset) : null,
      coverage: mockLogCoverage(input.sessionId),
    };
  },
  async getSessionLogRecord(recordId) {
    return logFixtures.find((entry) => entry.id === recordId) ?? null;
  },
  async exportSessionLogs() {
    return { status: "unavailable", path: null };
  },
};

/**
 * Which correlation each filter reads off a fixture record.
 *
 * Named here rather than derived from the key, because the query field and the context key are two
 * separate vocabularies that only happen to look alike — and a mapping that guessed would silently
 * match nothing the day one of them changed.
 */
const MOCK_CORRELATION_KEYS = {
  seatId: "seatId",
  runId: "runId",
  traceId: "traceId",
  spanId: "spanId",
  operationId: "operationId",
  agentId: "agentId",
} as const;

/**
 * A record matches only when it carries the correlation asked for.
 *
 * A record emitted without one is excluded rather than admitted. The alternative reads as evidence:
 * a reader filtering by run would see records that have nothing to do with that run and conclude it
 * touched them.
 */
function matchesMockCorrelation(
  entry: SessionLogEntry,
  input: SessionLogQuery,
): boolean {
  for (const [field, contextKey] of Object.entries(MOCK_CORRELATION_KEYS)) {
    const wanted = input[field as keyof typeof MOCK_CORRELATION_KEYS];
    if (typeof wanted !== "string" || wanted.trim().length === 0) continue;
    if (entry.context[contextKey] !== wanted) return false;
  }
  return true;
}

/**
 * Coverage the browser build can actually be driven through.
 *
 * The design requires the mock to exercise all four states, and a mock that always answered
 * `complete` would make the browser the one runtime where the incomplete-coverage rendering is
 * never seen. Selected by a session-id suffix so it is deterministic and needs no extra API: a
 * fixture session named `…#partial` reports `partial`, and anything else reports `complete`.
 */
function mockLogCoverage(sessionId: string): SessionLogCoverage {
  const requested = sessionId.split("#")[1];
  const state: SessionLogCoverageState =
    requested === "indexing" || requested === "partial" || requested === "unavailable"
      ? requested
      : "complete";
  return {
    state,
    droppedCount: state === "partial" ? 3 : 0,
    truncated: false,
    reasonCodes: state === "partial" ? ["log_receipt_dropped"] : [],
  };
}
