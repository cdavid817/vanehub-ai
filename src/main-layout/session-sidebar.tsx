import { useEffect, useMemo, useRef, useState, type DragEvent, type MouseEvent } from "react";
import { Archive, CheckSquare, ChevronDown, ChevronRight, FolderOpen, List, ListTree, Plus, Search, Trash2, X } from "lucide-react";
import { useTranslation } from "react-i18next";
import { ApplicationDialog } from "../components/ui/application-dialog";
import { Button } from "../components/ui/button";
import { lifecycleLabelKey } from "../lib/session-lifecycle";
import { cn } from "../lib/utils";
import { FilterPopover, type FilterField } from "../ui/filter-popover/FilterPopover";
import type { Session, SessionCategory, SessionSearchResult } from "../types/agent";
import { SessionCard } from "./session-card";
import { SessionCategoryGroup } from "./session-category-group";
import { SessionRowList } from "./session-row-list";
import {
  ALL_PROJECTS_FILTER,
  filterSearchResultsByAgent,
  filterSessionsByAgent,
  filterSessionsByDate,
  filterSessionsByProject,
  filterSessionsBySource,
  filterSessionsByStatus,
  groupSessionsByCategory,
  groupSessionsByProject,
  pruneSelectionToVisible,
  sessionAgentFilters,
  sessionDateFilters,
  sessionProjectFilterOptions,
  sessionSourceFilters,
  sessionStatusFilters,
  sortSessionsByAttention,
  type SessionAgentFilter,
  type SessionDateFilter,
  type SessionPresentationMode,
  type SessionSourceFilter,
  type SessionSourceMode,
  type SessionStatusFilter,
} from "./session-sidebar-model";

const sessionSidebarPresentationKey = "vanehub.session-sidebar.presentation.v1";
const sessionSidebarExpansionKey = "vanehub.session-sidebar.expanded-groups.v1";
const sessionSidebarSourceModeKey = "vanehub.session-sidebar.source-mode.v1";

function readPresentation(): SessionPresentationMode {
  const stored = typeof localStorage === "undefined" ? null : localStorage.getItem(sessionSidebarPresentationKey);
  return stored === "category" || stored === "project" || stored === "list" ? stored : "list";
}

// Persisted because a closed sidebar now unmounts (destination-layout) rather than just hiding.
function readSourceMode(): SessionSourceMode {
  const stored = typeof localStorage === "undefined" ? null : localStorage.getItem(sessionSidebarSourceModeKey);
  return stored === "archived" ? "archived" : "active";
}

function readExpandedGroups(): Set<string> {
  try {
    const parsed = JSON.parse(localStorage.getItem(sessionSidebarExpansionKey) ?? "[]") as unknown;
    return new Set(Array.isArray(parsed) ? parsed.filter((item): item is string => typeof item === "string") : []);
  } catch {
    return new Set();
  }
}

export function SessionSidebar({ activeSessionId, agentsAvailable, archivedSessions, categories, deletingSessions, focusSearchToken = 0, onAssignCategory, onBatchDelete, onContextMenu, onNew, onSearchChange, onSelect, searchQuery, searchResults, sessions }: {
  activeSessionId: string | null; agentsAvailable: boolean; archivedSessions: Session[]; categories: SessionCategory[]; deletingSessions?: boolean;
  /** Incremented by the shell to move focus here from the top bar search entry. */
  focusSearchToken?: number;
  onAssignCategory: (session: Session, categoryId: string | null) => void;
  onBatchDelete: (sessions: Session[]) => void;
  onContextMenu: (event: MouseEvent<HTMLButtonElement>, session: Session) => void;
  onNew: () => void; onSearchChange: (value: string) => void; onSelect: (session: Session) => void; searchQuery: string; searchResults: SessionSearchResult[]; sessions: Session[];
}) {
  const { t } = useTranslation();
  const searchInputRef = useRef<HTMLInputElement>(null);
  const [sourceMode, setSourceMode] = useState<SessionSourceMode>(readSourceMode);
  const [presentation, setPresentation] = useState<SessionPresentationMode>(readPresentation);
  const [agentFilter, setAgentFilter] = useState<SessionAgentFilter>("all");
  const [statusFilter, setStatusFilter] = useState<SessionStatusFilter>("all");
  const [sourceFilter, setSourceFilter] = useState<SessionSourceFilter>("all");
  const [dateFilter, setDateFilter] = useState<SessionDateFilter>("all");
  const [projectFilter, setProjectFilter] = useState(ALL_PROJECTS_FILTER);
  const [batchMode, setBatchMode] = useState(false);
  const [selectedIds, setSelectedIds] = useState<Set<string>>(() => new Set());
  const [confirmOpen, setConfirmOpen] = useState(false);
  const [expanded, setExpanded] = useState<Set<string>>(readExpandedGroups);
  const sourceSessions = sourceMode === "archived" ? archivedSessions : sessions;
  const filteredSessions = useMemo(() => {
    const byAgent = filterSessionsByAgent(sourceSessions, agentFilter);
    const byStatus = filterSessionsByStatus(byAgent, statusFilter);
    const bySource = filterSessionsBySource(byStatus, sourceFilter);
    const byDate = filterSessionsByDate(bySource, dateFilter, Date.now());
    return filterSessionsByProject(byDate, projectFilter);
  }, [agentFilter, dateFilter, projectFilter, sourceFilter, sourceSessions, statusFilter]);
  const filteredSearchResults = useMemo(() => filterSearchResultsByAgent(searchResults, agentFilter, sourceMode), [agentFilter, searchResults, sourceMode]);
  const renderedSessions = useMemo(() => searchQuery.trim() ? filteredSearchResults.map((result) => result.session) : filteredSessions, [filteredSearchResults, filteredSessions, searchQuery]);
  const selectedSessions = useMemo(() => renderedSessions.filter((session) => selectedIds.has(session.id)), [renderedSessions, selectedIds]);
  // 7.3: attention-first — needs-review, running, pinned, recently-updated, then the rest — one
  // ranked list rather than a pinned section unconditionally ahead of a running-but-unpinned
  // session (spec's own "Sort activity groups by priority" ranks pinned *below* running).
  const attentionSorted = useMemo(() => sourceMode === "active" && !searchQuery.trim() ? sortSessionsByAttention(filteredSessions, Date.now()) : renderedSessions, [filteredSessions, renderedSessions, searchQuery, sourceMode]);
  const categoryGroups = useMemo(() => groupSessionsByCategory(renderedSessions, categories, t("layout.uncategorized")), [categories, renderedSessions, t]);
  const projectGroups = useMemo(() => groupSessionsByProject(renderedSessions, t("layout.ungroupedProject")), [renderedSessions, t]);
  const projectFilterOptions = useMemo(() => sessionProjectFilterOptions(sourceSessions, t("layout.ungroupedProject")), [sourceSessions, t]);
  // "No sessions" and "a filter hid them all" need different wording, otherwise the only way to
  // tell which one you are looking at is to reset every filter and check.
  const filtersActive = agentFilter !== "all" || statusFilter !== "all" || sourceFilter !== "all" || dateFilter !== "all" || projectFilter !== ALL_PROJECTS_FILTER;
  const emptyListMessage = sourceMode === "archived"
    ? t("layout.noArchived")
    : filtersActive && sourceSessions.length > 0
      ? t("layout.noSessionsForFilter")
      : t("layout.noSessionsVisible");

  useEffect(() => {
    if (!focusSearchToken) return;
    searchInputRef.current?.focus();
    searchInputRef.current?.select();
  }, [focusSearchToken]);

  useEffect(() => {
    localStorage.setItem(sessionSidebarPresentationKey, presentation);
    localStorage.setItem(sessionSidebarSourceModeKey, sourceMode);
    localStorage.setItem(sessionSidebarExpansionKey, JSON.stringify([...expanded].sort()));
  }, [expanded, presentation, sourceMode]);

  useEffect(() => {
    if (!batchMode) {
      setSelectedIds(new Set());
      return;
    }
    setSelectedIds((current) => pruneSelectionToVisible(current, renderedSessions));
  }, [batchMode, renderedSessions]);

  const toggleSelected = (session: Session, checked: boolean) => setSelectedIds((current) => {
    const next = new Set(current);
    if (checked) next.add(session.id);
    else next.delete(session.id);
    return next;
  });
  const selectVisible = () => setSelectedIds(new Set(renderedSessions.map((session) => session.id)));
  const exitBatch = () => { setBatchMode(false); setConfirmOpen(false); setSelectedIds(new Set()); };
  const confirmDelete = () => { onBatchDelete(selectedSessions); exitBatch(); };
  function toggle(group: string) {
    setExpanded((current) => {
      const next = new Set(current);
      if (next.has(group)) next.delete(group);
      else next.add(group);
      return next;
    });
  }
  // 7.15: `dragOverGroupKey` drives the visible drop-target highlight while a drag is over a
  // section; `justDroppedGroupKey` drives a brief success flash once the drop lands. Neither is an
  // optimistic move — the row itself only reflects a new category once `onAssignCategory`'s own
  // mutation actually succeeds (use-main-layout-model.ts's `invalidateSessions`) — so a rejected
  // assignment has nothing to visually roll back; its failure now surfaces through that mutation's
  // own `onError` toast instead of failing silently.
  const [dragOverGroupKey, setDragOverGroupKey] = useState<string | null>(null);
  const [justDroppedGroupKey, setJustDroppedGroupKey] = useState<string | null>(null);
  const dropCategory = (event: DragEvent<HTMLElement>, categoryId: string | null, groupKey: string) => {
    event.preventDefault();
    setDragOverGroupKey(null);
    if (batchMode) return;
    const sessionId = event.dataTransfer.getData("text/plain");
    const session = renderedSessions.find((candidate) => candidate.id === sessionId);
    if (!session) return;
    onAssignCategory(session, categoryId);
    setJustDroppedGroupKey(groupKey);
    window.setTimeout(() => setJustDroppedGroupKey((current) => current === groupKey ? null : current), 600);
  };
  const card = (session: Session) => (
    <SessionCard
      active={activeSessionId === session.id}
      batchMode={batchMode}
      checked={selectedIds.has(session.id)}
      draggable={!batchMode && presentation === "category"}
      key={session.id}
      onContextMenu={(event) => onContextMenu(event, session)}
      onDragStart={(event) => event.dataTransfer.setData("text/plain", session.id)}
      onOpenActions={(event) => onContextMenu(event, session)}
      onSelect={() => onSelect(session)}
      onToggleChecked={(checked) => toggleSelected(session, checked)}
      session={session}
    />
  );

  const filterFields: FilterField[] = [
    { id: "agent", label: t("layout.agentFilter"), value: agentFilter, defaultValue: "all", onChange: (value) => setAgentFilter(value as SessionAgentFilter), options: sessionAgentFilters.map((value) => ({ value, label: t(`layout.agentFilter.${value}`) })) },
    { id: "status", label: t("layout.statusFilter"), value: statusFilter, defaultValue: "all", onChange: (value) => setStatusFilter(value as SessionStatusFilter), options: sessionStatusFilters.map((value) => ({ value, label: value === "all" ? t("layout.statusFilter.all") : t(lifecycleLabelKey(value)) })) },
    { id: "source", label: t("layout.sourceFilter"), value: sourceFilter, defaultValue: "all", onChange: (value) => setSourceFilter(value as SessionSourceFilter), options: sessionSourceFilters.map((value) => ({ value, label: t(`layout.sourceFilter.${value}`) })) },
    { id: "date", label: t("layout.dateFilter"), value: dateFilter, defaultValue: "all", onChange: (value) => setDateFilter(value as SessionDateFilter), options: sessionDateFilters.map((value) => ({ value, label: t(`layout.dateFilter.${value}`) })) },
    { id: "project", label: t("layout.projectFilter"), value: projectFilter, defaultValue: ALL_PROJECTS_FILTER, onChange: setProjectFilter, options: [{ value: ALL_PROJECTS_FILTER, label: t("layout.projectFilter.all") }, ...projectFilterOptions] },
  ];

  return (
    <aside className="flex h-full min-h-0 w-full flex-col bg-[hsl(var(--panel-muted))] py-3 pl-3 pr-1.5" data-testid="session-sidebar" onContextMenu={(event) => event.preventDefault()}>
      <div className="mb-3 flex items-center justify-between gap-2">
        <h2 className="text-sm font-semibold">{t("layout.sessions")}</h2>
        <Button className="h-7 px-2 text-xs" disabled={!agentsAvailable || batchMode} onClick={onNew}><Plus aria-hidden="true" className="h-3.5 w-3.5" />{t("layout.new")}</Button>
      </div>
      <label className="relative mb-2 block"><Search className="pointer-events-none absolute left-2 top-2 h-4 w-4 text-muted-foreground" aria-hidden="true" /><input className="ucd-input h-8 w-full rounded-md pl-8 pr-2 text-xs" id="workspace-session-search" onChange={(event) => onSearchChange(event.target.value)} placeholder={t("layout.sessionSearchPlaceholder")} ref={searchInputRef} value={searchQuery} /></label>
      {/* 7.6/7.13: search and New Session (above) are the only permanently prominent controls —
          everything else is one bounded, wrapping toolbar row rather than a stack of full-width
          rows, and the archive/batch toggles below are compact buttons rather than a menu a
          reader has to open first to discover they exist. */}
      <div className="mb-2 flex flex-wrap items-center gap-1">
        <div className="ucd-segmented grid grid-cols-3 gap-1 rounded-md p-1">
          <button className={cn("h-7 rounded text-xs", presentation === "list" ? "bg-background font-semibold text-primary" : "text-muted-foreground hover:bg-muted")} onClick={() => setPresentation("list")} type="button"><List className="mr-1 inline h-3.5 w-3.5" />{t("layout.sessionViewList")}</button>
          <button className={cn("h-7 rounded text-xs", presentation === "category" ? "bg-background font-semibold text-primary" : "text-muted-foreground hover:bg-muted")} onClick={() => setPresentation("category")} type="button"><ListTree className="mr-1 inline h-3.5 w-3.5" />{t("layout.sessionViewCategory")}</button>
          <button className={cn("h-7 rounded text-xs", presentation === "project" ? "bg-background font-semibold text-primary" : "text-muted-foreground hover:bg-muted")} onClick={() => setPresentation("project")} type="button"><FolderOpen className="mr-1 inline h-3.5 w-3.5" />{t("layout.sessionViewProject")}</button>
        </div>
        <FilterPopover fields={filterFields} triggerLabel={t("layout.sessionFilters")} />
        <button
          aria-label={t("layout.archiveToggle", { count: archivedSessions.length })}
          aria-pressed={sourceMode === "archived"}
          className={cn("flex h-7 items-center gap-1 rounded-md px-2 text-xs", sourceMode === "archived" ? "bg-background font-semibold text-primary" : "text-muted-foreground hover:bg-muted")}
          data-testid="session-archive-toggle"
          onClick={() => setSourceMode((mode) => mode === "archived" ? "active" : "archived")}
          title={t("layout.archiveToggle", { count: archivedSessions.length })}
          type="button"
        >
          <Archive aria-hidden="true" className="h-3.5 w-3.5" />
          {sourceMode === "archived" ? t("layout.archived") : archivedSessions.length}
        </button>
        {!batchMode ? (
          <button className="grid h-7 w-7 place-items-center rounded-md text-muted-foreground hover:bg-muted" onClick={() => setBatchMode(true)} title={t("layout.batchManage")} type="button">
            <CheckSquare aria-hidden="true" className="h-3.5 w-3.5" />
            <span className="sr-only">{t("layout.batchManage")}</span>
          </button>
        ) : null}
      </div>
      <div className="-mx-1 min-h-0 flex-1 overflow-x-hidden overflow-y-auto px-1">
        {searchQuery.trim() && presentation !== "project" ? <div className="grid gap-2"><div className="flex justify-between text-xs text-muted-foreground"><span>{t("layout.searchResults")}</span><span>{filteredSearchResults.length}</span></div>{filteredSearchResults.map((result) => (
          <div className="grid gap-1" key={result.session.id}>
            {card(result.session)}
            {/* 7.12: a "title" match's excerpt would just repeat the title already shown above it
                — only project/message matches add context the card itself does not already show. */}
            {result.matches[0] && result.matches[0].kind !== "title" ? <p className="truncate px-2 text-xs text-muted-foreground">{result.matches[0].excerpt}</p> : null}
          </div>
        ))}{filteredSearchResults.length === 0 ? <p className="ucd-muted-panel rounded-md p-3 text-xs text-muted-foreground">{t("layout.noSearchResults")}</p> : null}</div> : null}
        {!searchQuery.trim() && presentation === "list" ? (
          attentionSorted.length === 0
            ? <p className="ucd-muted-panel rounded-md p-3 text-xs text-muted-foreground">{emptyListMessage}</p>
            : <SessionRowList activeSessionId={activeSessionId} ariaLabel={t("layout.sessions")} card={card} sessions={attentionSorted} />
        ) : null}
        {!searchQuery.trim() && presentation === "category" ? <div className="grid gap-2">{categoryGroups.map((group) => (
          <SessionCategoryGroup
            activeSessionId={activeSessionId}
            batchMode={batchMode}
            card={card}
            dragOverGroupKey={dragOverGroupKey}
            expanded={expanded.has(`category:${group.id ?? "none"}`)}
            group={group}
            justDroppedGroupKey={justDroppedGroupKey}
            key={group.id ?? "uncategorized"}
            onDrop={dropCategory}
            onToggle={() => toggle(`category:${group.id ?? "none"}`)}
            setDragOverGroupKey={setDragOverGroupKey}
          />
        ))}</div> : null}
        {presentation === "project" ? <div className="grid gap-2">{projectGroups.map((group) => <section className="grid gap-2" key={group.id}><button className="ucd-list-row flex h-8 items-center gap-2 rounded-md px-2 text-left text-xs" onClick={() => toggle(group.id)} title={group.path ?? group.label} type="button">{expanded.has(group.id) ? <ChevronDown className="h-3.5 w-3.5" /> : <ChevronRight className="h-3.5 w-3.5" />}<FolderOpen className="h-3.5 w-3.5 text-primary" /><span className="truncate">{group.label}</span><span className="ml-auto">{group.sessions.length}</span></button>{expanded.has(group.id) ? <SessionRowList activeSessionId={activeSessionId} ariaLabel={group.label} card={card} sessions={group.sessions} /> : null}</section>)}{projectGroups.length === 0 ? <p className="ucd-muted-panel rounded-md p-3 text-xs text-muted-foreground">{searchQuery.trim() ? t("layout.noSearchResults") : sourceMode === "archived" ? t("layout.noArchived") : t("layout.noSessionsVisible")}</p> : null}</div> : null}
      </div>
      {/* 7.7: a dedicated region at the bottom of the pane, not a top-of-list panel — visible only
          in batch mode, so it never competes with the search/new-session controls above. */}
      {batchMode ? (
        <div className="ucd-muted-panel mt-2 grid gap-2 rounded-md p-2">
          <div className="flex items-center justify-between text-xs text-muted-foreground"><span>{t("layout.batchSelectedCount", { count: selectedSessions.length })}</span><span>{renderedSessions.length}</span></div>
          <div className="grid grid-cols-3 gap-1">
            <Button className="h-7 px-1 text-xs" disabled={renderedSessions.length === 0} onClick={selectVisible} size="sm" variant="outline">{t("layout.batchSelectVisible")}</Button>
            <Button className="h-7 px-1 text-xs text-destructive" disabled={selectedSessions.length === 0 || deletingSessions} onClick={() => setConfirmOpen(true)} size="sm" variant="outline"><Trash2 aria-hidden="true" className="h-3.5 w-3.5" />{t("layout.batchDelete")}</Button>
            <Button className="h-7 px-1 text-xs" onClick={exitBatch} size="sm" variant="outline"><X aria-hidden="true" className="h-3.5 w-3.5" />{t("layout.batchExit")}</Button>
          </div>
        </div>
      ) : null}
      {confirmOpen ? (
        <ApplicationDialog
          closeDisabled={deletingSessions}
          description={t("layout.batchDeleteDescription", { count: selectedSessions.length })}
          footer={(
            <div className="grid grid-cols-2 gap-2">
              <Button disabled={deletingSessions} onClick={() => setConfirmOpen(false)} size="sm" variant="outline">{t("layout.cancel")}</Button>
              <Button className="bg-destructive text-destructive-foreground" data-dialog-autofocus disabled={deletingSessions} onClick={confirmDelete} size="sm">{t("layout.delete")}</Button>
            </div>
          )}
          maxWidth="max-w-sm"
          onClose={() => setConfirmOpen(false)}
          title={t("layout.batchDeleteSessions")}
        >
          <p className="text-xs text-muted-foreground">{t("layout.batchDeleteHint")}</p>
        </ApplicationDialog>
      ) : null}
    </aside>
  );
}
