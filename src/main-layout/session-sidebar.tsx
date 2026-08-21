import { useEffect, useMemo, useRef, useState, type ChangeEvent, type DragEvent, type MouseEvent } from "react";
import { Archive, CheckSquare, ChevronDown, ChevronRight, EllipsisVertical, FolderOpen, List, ListTree, Pin, Plus, Search, Trash2, UsersRound, X } from "lucide-react";
import { useTranslation } from "react-i18next";
import { AgentBrandIcon } from "../components/agent-brand-icon";
import { ApplicationDialog } from "../components/ui/application-dialog";
import { Button } from "../components/ui/button";
import { getAgentVisualIdentity } from "../lib/agent-visual-identity";
import { lifecycleDotClass, lifecycleLabelKey, lifecycleTone } from "../lib/session-lifecycle";
import { cn } from "../lib/utils";
import type { Session, SessionCategory, SessionSearchResult } from "../types/agent";
import {
  filterSearchResultsByAgent,
  filterSessionsByAgent,
  groupSessionsByProject,
  pruneSelectionToVisible,
  sessionAgentFilters,
  type SessionAgentFilter,
  type SessionPresentationMode,
  type SessionSourceMode,
} from "./session-sidebar-model";

const sessionSidebarPresentationKey = "vanehub.session-sidebar.presentation.v1";
const sessionSidebarExpansionKey = "vanehub.session-sidebar.expanded-groups.v1";

function readPresentation(): SessionPresentationMode {
  if (typeof localStorage === "undefined") return "list";
  const stored = localStorage.getItem(sessionSidebarPresentationKey);
  return stored === "category" || stored === "project" || stored === "list" ? stored : "list";
}

function readExpandedGroups(): Set<string> {
  if (typeof localStorage === "undefined") return new Set();
  try {
    const parsed = JSON.parse(localStorage.getItem(sessionSidebarExpansionKey) ?? "[]") as unknown;
    return new Set(Array.isArray(parsed) ? parsed.filter((item): item is string => typeof item === "string") : []);
  } catch {
    return new Set();
  }
}

function SessionCard({ active, batchMode, checked, draggable, onContextMenu, onDragStart, onSelect, onToggleChecked, session }: {
  active: boolean; batchMode: boolean; checked: boolean; draggable?: boolean;
  onContextMenu: (event: MouseEvent<HTMLButtonElement>) => void;
  onDragStart?: (event: DragEvent<HTMLButtonElement>) => void;
  onSelect: () => void; onToggleChecked: (checked: boolean) => void; session: Session;
}) {
  const { i18n, t } = useTranslation();
  const meta = getAgentVisualIdentity(session.agentId);
  const activeSeatCount = session.seats?.filter((seat) => seat.leftAt == null).length ?? 1;
  const tone = lifecycleTone(session.lifecycleState);
  const date = new Intl.DateTimeFormat(i18n.language, { month: "2-digit", day: "2-digit" }).format(new Date(session.updatedAt));
  const select = () => {
    if (batchMode) onToggleChecked(!checked);
    else onSelect();
  };
  const checkboxChanged = (event: ChangeEvent<HTMLInputElement>) => {
    event.stopPropagation();
    onToggleChecked(event.target.checked);
  };
  return (
    <button
      aria-pressed={batchMode ? checked : active}
      className={cn(
        "relative w-full rounded-md border border-transparent px-3 py-2.5 text-left transition-colors hover:bg-[hsl(var(--panel-hover))] focus-visible:outline-hidden focus-visible:ring-2 focus-visible:ring-inset focus-visible:ring-ring",
        active && !batchMode && "border-border/70 bg-background shadow-xs",
        checked && batchMode && "border-primary/50 bg-[hsl(var(--nav-active-soft))]",
      )}
      data-session-id={session.id}
      draggable={draggable}
      onClick={select}
      onContextMenu={batchMode ? (event) => event.preventDefault() : onContextMenu}
      onDragStart={onDragStart}
      type="button"
    >
      {active && !batchMode ? <span className="absolute inset-y-2 left-0 w-0.5 rounded bg-primary" /> : null}
      <div className="flex min-w-0 items-center gap-2">
        {batchMode ? <input aria-label={t("layout.batchSelectSession")} checked={checked} className="h-4 w-4 shrink-0 accent-[hsl(var(--primary))]" onChange={checkboxChanged} onClick={(event) => event.stopPropagation()} type="checkbox" /> : null}
        <span className={cn("flex h-9 w-9 shrink-0 items-center justify-center rounded-lg border", meta.tone)} title={meta.label}><AgentBrandIcon agentId={session.agentId} className="h-5 w-5" /></span>
        <span className={cn("min-w-0 flex-1 truncate text-sm font-medium", session.archived && "text-muted-foreground")}>{session.title}</span>
        {session.pinned ? <Pin aria-hidden="true" className="h-3.5 w-3.5 shrink-0 text-primary" /> : null}
      </div>
      <div className="mt-1 flex min-w-0 items-center gap-1.5 overflow-hidden pl-11 text-[11px] text-muted-foreground">
        <span className={cn("h-2 w-2 shrink-0 rounded-full", session.archived ? "bg-muted-foreground" : lifecycleDotClass[tone])} />
        <span className="min-w-0 truncate">{session.archived ? t("layout.archived") : t(lifecycleLabelKey(session.lifecycleState))}</span>
        {activeSeatCount > 1 ? (
          <span
            className="inline-flex h-5 shrink-0 items-center gap-1 rounded-md border border-primary/30 bg-[hsl(var(--nav-active-soft))] px-1.5 text-[10px] font-semibold leading-none text-primary"
            data-testid="multi-agent-session-badge"
            title={t("session.participantCount", { count: activeSeatCount })}
          >
            <UsersRound aria-hidden="true" className="h-3 w-3" strokeWidth={2.25} />
            {t("createSession.agentMode.multi")}
          </span>
        ) : <span className="min-w-0 truncate font-mono">{meta.label}</span>}
        <span className="ml-auto shrink-0 font-mono">{date}</span>
      </div>
    </button>
  );
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
  const [sourceMode, setSourceMode] = useState<SessionSourceMode>("active");
  const [presentation, setPresentation] = useState<SessionPresentationMode>(readPresentation);
  const [agentFilter, setAgentFilter] = useState<SessionAgentFilter>("all");
  const [batchMode, setBatchMode] = useState(false);
  const [selectedIds, setSelectedIds] = useState<Set<string>>(() => new Set());
  const [confirmOpen, setConfirmOpen] = useState(false);
  const [expanded, setExpanded] = useState<Set<string>>(readExpandedGroups);
  const [moreMenuOpen, setMoreMenuOpen] = useState(false);
  const moreMenuRef = useRef<HTMLDivElement>(null);
  const moreMenuButtonRef = useRef<HTMLButtonElement>(null);
  const sourceSessions = sourceMode === "archived" ? archivedSessions : sessions;
  const filteredSessions = useMemo(() => filterSessionsByAgent(sourceSessions, agentFilter), [agentFilter, sourceSessions]);
  const filteredSearchResults = useMemo(() => filterSearchResultsByAgent(searchResults, agentFilter, sourceMode), [agentFilter, searchResults, sourceMode]);
  const renderedSessions = useMemo(() => searchQuery.trim() ? filteredSearchResults.map((result) => result.session) : filteredSessions, [filteredSearchResults, filteredSessions, searchQuery]);
  const selectedSessions = useMemo(() => renderedSessions.filter((session) => selectedIds.has(session.id)), [renderedSessions, selectedIds]);
  const pinned = useMemo(() => sourceMode === "active" && !searchQuery.trim() ? filteredSessions.filter((session) => session.pinned) : [], [filteredSessions, searchQuery, sourceMode]);
  const listSessions = useMemo(() => sourceMode === "active" && !searchQuery.trim() ? filteredSessions.filter((session) => !session.pinned) : filteredSessions, [filteredSessions, searchQuery, sourceMode]);
  const groupPool = useMemo(() => sourceMode === "active" && !searchQuery.trim() ? filteredSessions.filter((session) => !session.pinned) : renderedSessions, [filteredSessions, renderedSessions, searchQuery, sourceMode]);
  const categoryGroups = useMemo(() => [
    ...categories.map((category) => ({ id: category.id, label: category.name, sessions: groupPool.filter((session) => session.categoryId === category.id) })),
    { id: null, label: t("layout.uncategorized"), sessions: groupPool.filter((session) => !session.categoryId) },
  ], [categories, groupPool, t]);
  const projectGroups = useMemo(() => groupSessionsByProject(groupPool, t("layout.ungroupedProject")), [groupPool, t]);
  // "No sessions" and "the Agent filter hid them all" need different wording, otherwise the
  // only way to tell which one you are looking at is to reset the filter and check.
  const emptyListMessage = sourceMode === "archived"
    ? t("layout.noArchived")
    : agentFilter !== "all" && sourceSessions.length > 0
      ? t("layout.noSessionsForFilter")
      : t("layout.noSessionsVisible");

  useEffect(() => {
    if (!focusSearchToken) return;
    searchInputRef.current?.focus();
    searchInputRef.current?.select();
  }, [focusSearchToken]);

  useEffect(() => {
    if (typeof localStorage !== "undefined") localStorage.setItem(sessionSidebarPresentationKey, presentation);
  }, [presentation]);

  useEffect(() => {
    if (typeof localStorage !== "undefined") localStorage.setItem(sessionSidebarExpansionKey, JSON.stringify([...expanded].sort()));
  }, [expanded]);

  useEffect(() => {
    if (!batchMode) {
      setSelectedIds(new Set());
      return;
    }
    setSelectedIds((current) => pruneSelectionToVisible(current, renderedSessions));
  }, [batchMode, renderedSessions]);

  useEffect(() => {
    if (!moreMenuOpen) return;
    function closeOnOutsidePointer(event: PointerEvent) {
      const target = event.target;
      if (target instanceof Node && moreMenuRef.current?.contains(target)) return;
      setMoreMenuOpen(false);
    }
    window.addEventListener("pointerdown", closeOnOutsidePointer);
    return () => window.removeEventListener("pointerdown", closeOnOutsidePointer);
  }, [moreMenuOpen]);

  const toggleSelected = (session: Session, checked: boolean) => setSelectedIds((current) => {
    const next = new Set(current);
    if (checked) next.add(session.id);
    else next.delete(session.id);
    return next;
  });
  const selectVisible = () => setSelectedIds(new Set(renderedSessions.map((session) => session.id)));
  const exitBatch = () => {
    setBatchMode(false);
    setConfirmOpen(false);
    setSelectedIds(new Set());
  };
  const confirmDelete = () => {
    onBatchDelete(selectedSessions);
    exitBatch();
  };
  function toggle(group: string) {
    setExpanded((current) => {
      const next = new Set(current);
      if (next.has(group)) next.delete(group);
      else next.add(group);
      return next;
    });
  }
  const dropCategory = (event: DragEvent<HTMLElement>, categoryId: string | null) => {
    event.preventDefault();
    if (batchMode) return;
    const sessionId = event.dataTransfer.getData("text/plain");
    const session = renderedSessions.find((candidate) => candidate.id === sessionId);
    if (session) onAssignCategory(session, categoryId);
  };
  const card = (session: Session) => <SessionCard active={activeSessionId === session.id} batchMode={batchMode} checked={selectedIds.has(session.id)} draggable={!batchMode && presentation === "category"} key={session.id} onContextMenu={(event) => onContextMenu(event, session)} onDragStart={(event) => event.dataTransfer.setData("text/plain", session.id)} onSelect={() => onSelect(session)} onToggleChecked={(checked) => toggleSelected(session, checked)} session={session} />;

  return (
    <aside className="flex h-full min-h-0 w-full flex-col bg-[hsl(var(--panel-muted))] py-3 pl-3 pr-1.5 max-[640px]:max-h-64" data-testid="session-sidebar" onContextMenu={(event) => event.preventDefault()}>
      <div className="mb-3 flex items-center justify-between gap-2">
        <h2 className="text-sm font-semibold">{t("layout.sessions")}</h2>
        <div className="flex items-center gap-1">
          <div className="relative" ref={moreMenuRef}>
            <button
              aria-expanded={moreMenuOpen}
              aria-haspopup="menu"
              aria-label={t("layout.sessionMoreActions")}
              className={cn(
                "grid h-7 w-7 place-items-center rounded-md text-muted-foreground hover:bg-muted hover:text-foreground focus-visible:outline-hidden focus-visible:ring-2 focus-visible:ring-ring",
                sourceMode === "archived" && "text-primary",
              )}
              onClick={() => setMoreMenuOpen((value) => !value)}
              ref={moreMenuButtonRef}
              title={t("layout.sessionMoreActions")}
              type="button"
            >
              <EllipsisVertical aria-hidden="true" className="h-3.5 w-3.5" />
            </button>
            {moreMenuOpen ? (
              <div
                className="absolute right-0 top-8 z-30 min-w-44 rounded-md border border-border bg-background p-1 shadow-xl"
                onKeyDown={(event) => {
                  if (event.key !== "Escape") return;
                  event.preventDefault();
                  setMoreMenuOpen(false);
                  moreMenuButtonRef.current?.focus();
                }}
                role="menu"
              >
                <button
                  className="flex w-full items-center gap-2 rounded px-2 py-2 text-left text-xs hover:bg-muted disabled:pointer-events-none disabled:opacity-40"
                  disabled={batchMode}
                  onClick={() => { setBatchMode(true); setMoreMenuOpen(false); }}
                  role="menuitem"
                  type="button"
                >
                  <CheckSquare aria-hidden="true" className="h-3.5 w-3.5" />
                  {batchMode ? t("layout.batchManaging") : t("layout.batchManage")}
                </button>
                <button
                  className="flex w-full items-center gap-2 rounded px-2 py-2 text-left text-xs hover:bg-muted"
                  onClick={() => { setSourceMode((mode) => mode === "archived" ? "active" : "archived"); setMoreMenuOpen(false); }}
                  role="menuitem"
                  type="button"
                >
                  <Archive aria-hidden="true" className="h-3.5 w-3.5" />
                  {sourceMode === "archived" ? t("layout.archived") : `${t("layout.archive")} ${archivedSessions.length}`}
                </button>
              </div>
            ) : null}
          </div>
          <Button className="h-7 px-2 text-xs" disabled={!agentsAvailable || batchMode} onClick={onNew}><Plus aria-hidden="true" className="h-3.5 w-3.5" />{t("layout.new")}</Button>
        </div>
      </div>
      <label className="relative mb-2 block"><Search className="pointer-events-none absolute left-2 top-2 h-4 w-4 text-muted-foreground" aria-hidden="true" /><input className="ucd-input h-8 w-full rounded-md pl-8 pr-2 text-xs" id="workspace-session-search" onChange={(event) => onSearchChange(event.target.value)} placeholder={t("layout.sessionSearchPlaceholder")} ref={searchInputRef} value={searchQuery} /></label>
      <div className="ucd-segmented mb-2 grid grid-cols-3 gap-1 rounded-md p-1">
        <button className={cn("h-7 rounded text-xs", presentation === "list" ? "bg-background font-semibold text-primary" : "text-muted-foreground hover:bg-muted")} onClick={() => setPresentation("list")} type="button"><List className="mr-1 inline h-3.5 w-3.5" />{t("layout.sessionViewList")}</button>
        <button className={cn("h-7 rounded text-xs", presentation === "category" ? "bg-background font-semibold text-primary" : "text-muted-foreground hover:bg-muted")} onClick={() => setPresentation("category")} type="button"><ListTree className="mr-1 inline h-3.5 w-3.5" />{t("layout.sessionViewCategory")}</button>
        <button className={cn("h-7 rounded text-xs", presentation === "project" ? "bg-background font-semibold text-primary" : "text-muted-foreground hover:bg-muted")} onClick={() => setPresentation("project")} type="button"><FolderOpen className="mr-1 inline h-3.5 w-3.5" />{t("layout.sessionViewProject")}</button>
      </div>
      <select aria-label={t("layout.agentFilter")} className="ucd-input mb-2 h-8 rounded-md px-2 text-xs" onChange={(event) => setAgentFilter(event.target.value as SessionAgentFilter)} value={agentFilter}>{sessionAgentFilters.map((filter) => <option key={filter} value={filter}>{t(`layout.agentFilter.${filter}`)}</option>)}</select>
      {batchMode ? <div className="ucd-muted-panel mb-2 grid gap-2 rounded-md p-2"><div className="flex items-center justify-between text-xs text-muted-foreground"><span>{t("layout.batchSelectedCount", { count: selectedSessions.length })}</span><span>{renderedSessions.length}</span></div><div className="grid grid-cols-3 gap-1"><Button className="h-7 px-1 text-xs" disabled={renderedSessions.length === 0} onClick={selectVisible} size="sm" variant="outline">{t("layout.batchSelectVisible")}</Button><Button className="h-7 px-1 text-xs text-destructive" disabled={selectedSessions.length === 0 || deletingSessions} onClick={() => setConfirmOpen(true)} size="sm" variant="outline"><Trash2 aria-hidden="true" className="h-3.5 w-3.5" />{t("layout.batchDelete")}</Button><Button className="h-7 px-1 text-xs" onClick={exitBatch} size="sm" variant="outline"><X aria-hidden="true" className="h-3.5 w-3.5" />{t("layout.batchExit")}</Button></div></div> : null}
      <div className="-mx-1 min-h-0 flex-1 overflow-x-hidden overflow-y-auto px-1">
        {searchQuery.trim() && presentation !== "project" ? <div className="grid gap-2"><div className="flex justify-between text-xs text-muted-foreground"><span>{t("layout.searchResults")}</span><span>{filteredSearchResults.length}</span></div>{filteredSearchResults.map((result) => <div className="grid gap-1" key={result.session.id}>{card(result.session)}<p className="truncate px-2 text-xs text-muted-foreground">{result.matches[0]?.excerpt}</p></div>)}{filteredSearchResults.length === 0 ? <p className="ucd-muted-panel rounded-md p-3 text-xs text-muted-foreground">{t("layout.noSearchResults")}</p> : null}</div> : null}
        {!searchQuery.trim() && pinned.length > 0 ? <section className="mb-3 grid gap-2 border-b border-border pb-3"><div className="flex justify-between text-xs text-muted-foreground"><span><Pin className="mr-1 inline h-3.5 w-3.5" />{t("layout.pinned")}</span><span>{pinned.length}</span></div>{pinned.map(card)}</section> : null}
        {!searchQuery.trim() && presentation === "list" ? <div className="grid gap-1">{listSessions.map(card)}{listSessions.length === 0 ? <p className="ucd-muted-panel rounded-md p-3 text-xs text-muted-foreground">{emptyListMessage}</p> : null}</div> : null}
        {!searchQuery.trim() && presentation === "category" ? <div className="grid gap-2">{categoryGroups.map((group) => <section className="grid gap-2" data-session-category-id={group.id ?? "uncategorized"} key={group.id ?? "uncategorized"} onDragOver={(event) => { if (!batchMode) event.preventDefault(); }} onDrop={(event) => dropCategory(event, group.id)}><button className="ucd-list-row flex h-8 items-center gap-2 rounded-md px-2 text-left text-xs" onClick={() => toggle(`category:${group.id ?? "none"}`)} type="button">{expanded.has(`category:${group.id ?? "none"}`) ? <ChevronDown className="h-3.5 w-3.5" /> : <ChevronRight className="h-3.5 w-3.5" />}<ListTree className="h-3.5 w-3.5 text-primary" /><span className="truncate">{group.label}</span><span className="ml-auto">{group.sessions.length}</span></button>{expanded.has(`category:${group.id ?? "none"}`) ? group.sessions.map(card) : null}</section>)}</div> : null}
        {presentation === "project" ? <div className="grid gap-2">{projectGroups.map((group) => <section className="grid gap-2" key={group.id}><button className="ucd-list-row flex h-8 items-center gap-2 rounded-md px-2 text-left text-xs" onClick={() => toggle(group.id)} title={group.path ?? group.label} type="button">{expanded.has(group.id) ? <ChevronDown className="h-3.5 w-3.5" /> : <ChevronRight className="h-3.5 w-3.5" />}<FolderOpen className="h-3.5 w-3.5 text-primary" /><span className="truncate">{group.label}</span><span className="ml-auto">{group.sessions.length}</span></button>{expanded.has(group.id) ? group.sessions.map(card) : null}</section>)}{projectGroups.length === 0 ? <p className="ucd-muted-panel rounded-md p-3 text-xs text-muted-foreground">{searchQuery.trim() ? t("layout.noSearchResults") : sourceMode === "archived" ? t("layout.noArchived") : t("layout.noSessionsVisible")}</p> : null}</div> : null}
      </div>
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
