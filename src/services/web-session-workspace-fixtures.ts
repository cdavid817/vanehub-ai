import type {
  DirectoryEntry,
  FileSearchMatch,
  GitDiffResult,
  GitStatusResult,
  SessionDocument,
  SessionLogEntry,
  SessionLogLevel,
} from "../types/session-workspace";

export const availableContext = { availability: "available" as const, rootName: "vanehub-demo", reason: null };

export const directoryFixtures: Record<string, DirectoryEntry[]> = {
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

// Line N reads `export const valueN`, so a preview that mislabels a line is visible at a
// glance — and long enough that selecting across it requires the line list to scroll.
const longModule = Array.from({ length: 400 }, (_, index) => `export const value${index + 1} = ${index + 1};`).join("\n");

export const fileFixtures: Record<string, string> = {
  "README.md": "# VaneHub Web Preview\n\nThis document is deterministic mock content for the session workspace.",
  "docs/architecture.md": "# Architecture\n\n- React service boundary\n- Tauri desktop adapter\n- Web mock adapter",
  "docs/notes.txt": "Web preview note: local filesystem operations are simulated.",
  "package.json": "{\n  \"name\": \"vanehub-web-preview\",\n  \"private\": true\n}",
  "src/main.ts": "export const runtime = \"web-mock\";\n",
  "src/long-module.ts": longModule,
};

export const documentFixtures: SessionDocument[] = [
  { name: "README.md", path: "README.md", kind: "markdown" },
  { name: "architecture.md", path: "docs/architecture.md", kind: "markdown" },
  { name: "notes.txt", path: "docs/notes.txt", kind: "text" },
];

// Mention candidates cover source files, not only the Markdown/text documents the
// Documents tab lists — the same distinction the native runtime makes.
export const searchFixtures: FileSearchMatch[] = Object.keys(fileFixtures).map((path) => ({
  name: path.split("/").pop() ?? path,
  path,
}));

export const statusFixture: GitStatusResult = {
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

export const diffFixture: GitDiffResult = {
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

const fixtureLogLevels: SessionLogLevel[] = ["info", "debug", "warn", "error"];

export const logFixtures: SessionLogEntry[] = [
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
  ...Array.from({ length: 597 }, (_, index): SessionLogEntry => {
    const sequence = 600 - index;
    const level = fixtureLogLevels[index % fixtureLogLevels.length];
    return {
      id: `web-log-${sequence}-history`,
      timestamp: new Date(Date.parse("2026-07-17T08:00:00.000Z") - index * 60_000).toISOString(),
      level,
      category: "session.history",
      message: `Deterministic Agent output ${sequence}.`,
      context: { fixture: "virtual-scroll", sequence: String(sequence) },
    };
  }),
];
