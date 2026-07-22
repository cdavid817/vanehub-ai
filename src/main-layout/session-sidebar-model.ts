import { folderNameFromPath, normalizeDisplayPath } from "../lib/session-path";
import type { Session, SessionSearchResult } from "../types/agent";

export type SessionAgentFilter = "all" | "claude-code" | "opencode" | "codex-cli" | "gemini-cli";
export type SessionPresentationMode = "list" | "category" | "project";
export type SessionSourceMode = "active" | "archived";

export interface SessionProjectGroup {
  id: string;
  label: string;
  path: string | null;
  sessions: Session[];
}

const ungroupedProjectKey = "project:none";

export const sessionAgentFilters: SessionAgentFilter[] = ["all", "claude-code", "opencode", "codex-cli", "gemini-cli"];

export function filterSessionsByAgent(sessions: Session[], agentFilter: SessionAgentFilter): Session[] {
  if (agentFilter === "all") return sessions;
  return sessions.filter((session) => session.agentId === agentFilter);
}

export function filterSearchResultsByAgent(results: SessionSearchResult[], agentFilter: SessionAgentFilter, sourceMode: SessionSourceMode): SessionSearchResult[] {
  return results.filter((result) => {
    const sourceMatches = sourceMode === "archived" ? result.session.archived : !result.session.archived;
    const agentMatches = agentFilter === "all" || result.session.agentId === agentFilter;
    return sourceMatches && agentMatches;
  });
}

export function pruneSelectionToVisible(selectedIds: Set<string>, visibleSessions: Session[]): Set<string> {
  const visibleIds = new Set(visibleSessions.map((session) => session.id));
  let changed = false;
  const next = new Set<string>();
  selectedIds.forEach((id) => {
    if (visibleIds.has(id)) next.add(id);
    else changed = true;
  });
  return changed ? next : selectedIds;
}

export function getSessionProjectGroupKey(session: Session): string {
  const path = session.worktreePath ?? session.remoteWorkspace?.uri ?? session.projectPath ?? session.folder;
  const normalized = path?.trim() ? normalizeDisplayPath(path.trim()) : null;
  return normalized ? `project:${normalized}` : ungroupedProjectKey;
}

export function getSessionProjectGroupLabel(session: Session, ungroupedLabel: string): string {
  if (session.worktreeName?.trim()) return session.worktreeName.trim();
  if (session.remoteWorkspace?.displayName.trim()) return session.remoteWorkspace.displayName.trim();
  const path = session.worktreePath ?? session.projectPath ?? session.folder;
  if (!path?.trim()) return ungroupedLabel;
  const normalized = normalizeDisplayPath(path.trim());
  return folderNameFromPath(normalized) || normalized;
}

export function groupSessionsByProject(sessions: Session[], ungroupedLabel: string): SessionProjectGroup[] {
  const groups: SessionProjectGroup[] = [];
  const byKey = new Map<string, SessionProjectGroup>();
  sessions.forEach((session) => {
    const id = getSessionProjectGroupKey(session);
    let group = byKey.get(id);
    if (!group) {
      const path = id === ungroupedProjectKey ? null : id.slice("project:".length);
      group = {
        id,
        label: getSessionProjectGroupLabel(session, ungroupedLabel),
        path,
        sessions: [],
      };
      byKey.set(id, group);
      groups.push(group);
    }
    group.sessions.push(session);
  });
  return groups;
}