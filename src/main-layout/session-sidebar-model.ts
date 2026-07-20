import type { Session, SessionSearchResult } from "../types/agent";

export type SessionAgentFilter = "all" | "claude-code" | "opencode" | "codex-cli" | "gemini-cli";
export type SessionPresentationMode = "list" | "category";
export type SessionSourceMode = "active" | "archived";

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
