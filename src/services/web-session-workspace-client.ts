import type { AgentService } from "./agent-service";
import { rankFileCandidates } from "./file-search-ranking";
import {
  availableContext,
  diffFixture,
  directoryFixtures,
  documentFixtures,
  fileFixtures,
  logFixtures,
  searchFixtures,
  statusFixture,
} from "./web-session-workspace-fixtures";
import { sessionWorkspaceLimits } from "../session-workspace/session-workspace-limits";
import type { FileContent, ShellEvent, ShellSession } from "../types/session-workspace";
import type { FolderOpenerAvailability, FolderOpenerId, FolderOpenerPreferences } from "../types/folder-opener";

type SessionWorkspaceMethods = Pick<
  AgentService,
  | "listSessionDirectory"
  | "readSessionFile"
  | "listSessionDocuments"
  | "searchSessionFiles"
  | "getSessionGitStatus"
  | "getSessionGitDiff"
  | "listSessionLogs"
  | "exportSessionLogs"
  | "createShell"
  | "writeShellInput"
  | "resetShellDirectory"
  | "resizeShell"
  | "killShell"
  | "subscribeShellEvents"
  | "listFolderOpeners"
  | "refreshFolderOpeners"
  | "getFolderOpenerPreferences"
  | "saveFolderOpenerPreferences"
  | "openSessionFolder"
  | "subscribeFolderOpenerEvents"
>;


let nextShellId = 1;
const shells = new Map<string, ShellSession>();
const shellSubscribers = new Map<string, Set<(event: ShellEvent) => void>>();

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

function publishShellEvent(event: ShellEvent) {
  shellSubscribers.get(event.shellId)?.forEach((handler) => handler(event));
}

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
    };
  },
  async exportSessionLogs() {
    return { status: "unavailable", path: null };
  },
  async createShell(input) {
    const shell: ShellSession = {
      shellId: `web-shell-${nextShellId}`,
      sessionId: input.sessionId,
      state: "connected",
      runtime: { kind: "simulated", supportsResize: false, supportsReplay: true, supportsReconnect: false },
    };
    nextShellId += 1;
    shells.set(shell.shellId, shell);
    return shell;
  },
  async writeShellInput(shellId, content) {
    const shell = shells.get(shellId);
    if (!shell) throw new Error(`Mock shell not found: ${shellId}`);
    publishShellEvent({
      type: "output",
      shellId,
      sessionId: shell.sessionId,
      content: `\r\n[WEB MOCK] ${content.replace(/[\r\n]+$/u, "")}\r\nmock> `,
    });
  },
  async resetShellDirectory(shellId) {
    const shell = shells.get(shellId);
    if (!shell) throw new Error(`Mock shell not found: ${shellId}`);
    publishShellEvent({ type: "output", shellId, sessionId: shell.sessionId, content: "\r\n[WEB MOCK] cd <session-root>\r\nmock> " });
  },
  async resizeShell(input) {
    if (!shells.has(input.shellId)) throw new Error(`Mock shell not found: ${input.shellId}`);
  },
  async killShell(shellId) {
    const shell = shells.get(shellId);
    if (!shell) return;
    shells.delete(shellId);
    publishShellEvent({ type: "state", shellId, sessionId: shell.sessionId, state: "disconnected" });
  },
  async subscribeShellEvents(shellId, handler) {
    const subscribers = shellSubscribers.get(shellId) ?? new Set<(event: ShellEvent) => void>();
    subscribers.add(handler);
    shellSubscribers.set(shellId, subscribers);
    return () => {
      const current = shellSubscribers.get(shellId);
      current?.delete(handler);
      if (current?.size === 0) shellSubscribers.delete(shellId);
    };
  },
};
