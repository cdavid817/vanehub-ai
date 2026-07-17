import type { GitChangeKind, GitStatusEntry } from "../types/session-workspace";

const statusCodes: Record<GitChangeKind, string> = {
  unmodified: " ",
  modified: "M",
  added: "A",
  deleted: "D",
  renamed: "R",
  copied: "C",
  untracked: "?",
  conflicted: "U",
};

export function gitStatusPresentation(entry: GitStatusEntry) {
  const kinds = [...new Set([entry.index, entry.worktree].filter((kind) => kind !== "unmodified"))];
  return {
    code: `${statusCodes[entry.index]}${statusCodes[entry.worktree]}`,
    kinds,
  };
}
