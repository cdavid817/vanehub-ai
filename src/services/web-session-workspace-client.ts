import type { AgentService } from "./agent-service";
import { sessionWorkspaceLimits } from "../session-workspace/session-workspace-limits";
import type {
  DirectoryEntry,
  FileContent,
  GitDiffResult,
  GitStatusResult,
  SessionDocument,
  SessionLogEntry,
  ShellEvent,
  ShellSession,
} from "../types/session-workspace";

type SessionWorkspaceMethods = Pick<
  AgentService,
  | "listSessionDirectory"
  | "readSessionFile"
  | "listSessionDocuments"
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
>;

const availableContext = { availability: "available" as const, rootName: "vanehub-demo", reason: null };

const directoryFixtures: Record<string, DirectoryEntry[]> = {
  "": [
    { name: "docs", path: "docs", kind: "directory", size: null },
    { name: "src", path: "src", kind: "directory", size: null },
    { name: "README.md", path: "README.md", kind: "file", size: 284 },
    { name: "package.json", path: "package.json", kind: "file", size: 192 },
  ],
  docs: [
    { name: "architecture.md", path: "docs/architecture.md", kind: "file", size: 412 },
    { name: "notes.txt", path: "docs/notes.txt", kind: "file", size: 96 },
  ],
  src: [{ name: "main.ts", path: "src/main.ts", kind: "file", size: 128 }],
};

const fileFixtures: Record<string, string> = {
  "README.md": "# VaneHub Web Preview\n\nThis document is deterministic mock content for the session workspace.",
  "docs/architecture.md": "# Architecture\n\n- React service boundary\n- Tauri desktop adapter\n- Web mock adapter",
  "docs/notes.txt": "Web preview note: local filesystem operations are simulated.",
  "package.json": "{\n  \"name\": \"vanehub-web-preview\",\n  \"private\": true\n}",
  "src/main.ts": "export const runtime = \"web-mock\";\n",
};

const documentFixtures: SessionDocument[] = [
  { name: "README.md", path: "README.md", kind: "markdown" },
  { name: "architecture.md", path: "docs/architecture.md", kind: "markdown" },
  { name: "notes.txt", path: "docs/notes.txt", kind: "text" },
];

const statusFixture: GitStatusResult = {
  context: availableContext,
  isGit: true,
  branch: "worktree/web-preview",
  items: [
    { path: "src/main.ts", previousPath: null, index: "unmodified", worktree: "modified" },
    { path: "docs/session-tabs.md", previousPath: null, index: "added", worktree: "unmodified" },
    { path: "notes.todo", previousPath: null, index: "untracked", worktree: "untracked" },
  ],
  truncated: false,
  nextCursor: null,
};

const diffFixture: GitDiffResult = {
  context: availableContext,
  source: "working",
  files: [
    {
      oldPath: "src/main.ts",
      newPath: "src/main.ts",
      binary: false,
      oversized: false,
      hunks: [
        {
          header: "@@ -1,1 +1,2 @@",
          oldStart: 1,
          oldLines: 1,
          newStart: 1,
          newLines: 2,
          lines: [
            { kind: "deletion", content: "export const runtime = \"web\";", oldLineNumber: 1, newLineNumber: null },
            { kind: "addition", content: "export const runtime = \"web-mock\";", oldLineNumber: null, newLineNumber: 1 },
            { kind: "addition", content: "export const simulated = true;", oldLineNumber: null, newLineNumber: 2 },
          ],
        },
      ],
    },
  ],
  truncated: false,
};

const logFixtures: SessionLogEntry[] = [
  {
    id: "web-log-3",
    timestamp: "2026-07-17T08:03:00.000Z",
    level: "warn",
    category: "session.runtime",
    message: "Simulated retry completed with [REDACTED] context.",
    context: { runtime: "web-mock" },
  },
  {
    id: "web-log-2",
    timestamp: "2026-07-17T08:02:00.000Z",
    level: "debug",
    category: "session.workspace",
    message: "Loaded deterministic project fixtures.",
    context: { runtime: "web-mock" },
  },
  {
    id: "web-log-1",
    timestamp: "2026-07-17T08:01:00.000Z",
    level: "info",
    category: "session.runtime",
    message: "Web preview session initialized.",
    context: { runtime: "web-mock" },
  },
];

let nextShellId = 1;
const shells = new Map<string, ShellSession>();
const shellSubscribers = new Map<string, Set<(event: ShellEvent) => void>>();

function publishShellEvent(event: ShellEvent) {
  shellSubscribers.get(event.shellId)?.forEach((handler) => handler(event));
}

export const webSessionWorkspaceClient: SessionWorkspaceMethods = {
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
      capability: "simulated",
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
