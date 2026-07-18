import { useMemo, useState, type DragEvent, type MouseEvent } from "react";
import { Archive, Bot, BrainCircuit, ChevronDown, ChevronRight, Code2, Folder, Pin, Plus, Search, Sparkles, Tags, TerminalSquare, type LucideIcon } from "lucide-react";
import { useTranslation } from "react-i18next";
import { Button } from "../components/ui/button";
import { cn } from "../lib/utils";
import type { Session, SessionCategory, SessionSearchResult } from "../types/agent";

type SidebarMode = "activity" | "group" | "category" | "archived";
type ActivityKey = "needs-input" | "pending-verification" | "in-progress" | "inactive";
type AgentKey = "codex" | "claude-code" | "opencode" | "gemini" | "unknown";

const activityGroups: Array<{ key: ActivityKey; labelKey: string }> = [
  { key: "needs-input", labelKey: "layout.needsInput" },
  { key: "pending-verification", labelKey: "layout.pendingVerification" },
  { key: "in-progress", labelKey: "layout.running" },
  { key: "inactive", labelKey: "layout.inactive" },
];

const agentMeta: Record<AgentKey, { label: string; Icon: LucideIcon; tone: string }> = {
  codex: { label: "Codex", Icon: Code2, tone: "ucd-agent-codex" },
  "claude-code": { label: "Claude Code", Icon: Sparkles, tone: "ucd-agent-claude" },
  opencode: { label: "OpenCode", Icon: TerminalSquare, tone: "ucd-agent-opencode" },
  gemini: { label: "Gemini", Icon: BrainCircuit, tone: "ucd-agent-gemini" },
  unknown: { label: "Agent", Icon: Bot, tone: "border-border bg-muted text-muted-foreground" },
};

function activityFor(session: Session): ActivityKey {
  if (session.archived || session.lifecycleState === "idle" || session.lifecycleState === "stopped") return "inactive";
  if (session.lifecycleState === "failed") return "needs-input";
  if (session.lifecycleState === "starting") return "pending-verification";
  return "in-progress";
}

function agentFor(session: Session): AgentKey {
  if (session.agentId.includes("codex")) return "codex";
  if (session.agentId.includes("claude")) return "claude-code";
  if (session.agentId.includes("opencode")) return "opencode";
  if (session.agentId.includes("gemini")) return "gemini";
  return "unknown";
}

function SessionCard({ active, draggable, onContextMenu, onDragStart, onSelect, session }: {
  active: boolean;
  draggable?: boolean;
  onContextMenu: (event: MouseEvent<HTMLButtonElement>) => void;
  onDragStart?: (event: DragEvent<HTMLButtonElement>) => void;
  onSelect: () => void;
  session: Session;
}) {
  const { i18n, t } = useTranslation();
  const meta = agentMeta[agentFor(session)];
  const lifecycle: Record<Session["lifecycleState"], string> = {
    failed: t("layout.needsInput"), idle: t("layout.idle"), running: t("layout.running"),
    starting: t("layout.pendingVerification"), stopped: t("layout.stopped"),
  };
  const date = new Intl.DateTimeFormat(i18n.language, { month: "2-digit", day: "2-digit" }).format(new Date(session.updatedAt));
  return (
    <button className={cn("ucd-list-row relative w-full rounded-lg p-2.5 text-left", active && "border-primary bg-[hsl(var(--nav-active-soft))]")} draggable={draggable} onClick={onSelect} onContextMenu={onContextMenu} onDragStart={onDragStart} type="button">
      {active ? <span className="absolute left-0 top-2 h-10 w-0.5 rounded bg-primary" /> : null}
      <div className="flex min-w-0 items-center gap-2">
        <span className={cn("flex h-6 w-6 shrink-0 items-center justify-center rounded border", meta.tone)} title={meta.label}><meta.Icon aria-hidden="true" className="h-3.5 w-3.5" /></span>
        <span className={cn("truncate text-sm font-medium", session.archived && "text-muted-foreground")}>{session.title}</span>
        {session.pinned ? <Pin aria-hidden="true" className="ml-auto h-3.5 w-3.5 text-primary" /> : null}
      </div>
      <div className="mt-2 flex items-center gap-2 text-xs text-muted-foreground">
        <span className={cn("h-2 w-2 rounded-full", session.archived ? "bg-muted-foreground" : "bg-[hsl(var(--success))]")} />
        <span>{session.archived ? t("layout.archived") : lifecycle[session.lifecycleState]}</span><span className="font-mono">{meta.label}</span><span className="ml-auto font-mono">{date}</span>
      </div>
    </button>
  );
}

export function SessionSidebar({ activeSessionId, agentsAvailable, archivedSessions, categories, onAssignCategory, onContextMenu, onNew, onSearchChange, onSelect, searchQuery, searchResults, sessions }: {
  activeSessionId: string | null; agentsAvailable: boolean; archivedSessions: Session[]; categories: SessionCategory[];
  onAssignCategory: (session: Session, categoryId: string | null) => void;
  onContextMenu: (event: MouseEvent<HTMLButtonElement>, session: Session) => void;
  onNew: () => void; onSearchChange: (value: string) => void; onSelect: (session: Session) => void; searchQuery: string; searchResults: SessionSearchResult[]; sessions: Session[];
}) {
  const { t } = useTranslation();
  const [mode, setMode] = useState<SidebarMode>("activity");
  const [expanded, setExpanded] = useState<Set<string>>(() => new Set());
  const pinned = useMemo(() => sessions.filter((session) => session.pinned), [sessions]);
  const activity = useMemo(() => activityGroups.map((group) => ({ ...group, sessions: sessions.filter((session) => !session.pinned && activityFor(session) === group.key) })), [sessions]);
  const folders = useMemo(() => {
    const result = new Map<string, Session[]>();
    sessions.filter((session) => !session.pinned).forEach((session) => {
      const folder = session.folder ?? t("layout.currentWorkspace");
      result.set(folder, [...(result.get(folder) ?? []), session]);
    });
    return [...result.entries()];
  }, [sessions, t]);
  const categoryGroups = useMemo(() => {
    const uncategorized = sessions.filter((session) => !session.pinned && !session.categoryId);
    return [
      ...categories.map((category) => ({ id: category.id, label: category.name, sessions: sessions.filter((session) => !session.pinned && session.categoryId === category.id) })),
      { id: null, label: t("layout.uncategorized"), sessions: uncategorized },
    ];
  }, [categories, sessions, t]);
  const card = (session: Session) => <SessionCard active={activeSessionId === session.id} draggable={mode === "category"} key={session.id} onContextMenu={(event) => onContextMenu(event, session)} onDragStart={(event) => event.dataTransfer.setData("text/plain", session.id)} onSelect={() => onSelect(session)} session={session} />;
  const dropCategory = (event: DragEvent<HTMLElement>, categoryId: string | null) => {
    event.preventDefault();
    const sessionId = event.dataTransfer.getData("text/plain");
    const session = sessions.find((candidate) => candidate.id === sessionId);
    if (session) onAssignCategory(session, categoryId);
  };
  function toggle(folder: string) {
    setExpanded((current) => {
      const next = new Set(current);
      if (next.has(folder)) next.delete(folder);
      else next.add(folder);
      return next;
    });
  }

  return (
    <aside className="ucd-panel flex h-full min-h-0 w-full flex-col rounded-lg p-3 max-[640px]:max-h-64" onContextMenu={(event) => event.preventDefault()}>
      <div className="mb-3 flex items-center justify-between gap-2"><h2 className="text-sm font-semibold">{t("layout.sessions")}</h2><Button className="h-7 px-2 text-xs" disabled={!agentsAvailable} onClick={onNew}><Plus aria-hidden="true" className="h-3.5 w-3.5" />{t("layout.new")}</Button></div>
      <label className="relative mb-3 block">
        <Search className="pointer-events-none absolute left-2 top-2 h-4 w-4 text-muted-foreground" aria-hidden="true" />
        <input className="ucd-input h-8 w-full rounded-md pl-8 pr-2 text-xs" onChange={(event) => onSearchChange(event.target.value)} placeholder={t("layout.sessionSearchPlaceholder")} value={searchQuery} />
      </label>
      <div className="ucd-segmented mb-3 grid grid-cols-4 gap-1 rounded-md p-1">
        {(["activity", "group", "category", "archived"] as const).map((item) => <button className={cn("h-7 rounded text-xs", mode === item ? "bg-background font-semibold text-primary" : "text-muted-foreground hover:bg-muted")} key={item} onClick={() => setMode(item)} type="button">{item === "activity" ? t("layout.activity") : item === "group" ? t("layout.group") : item === "category" ? t("layout.categories") : `${t("layout.archive")} ${archivedSessions.length}`}</button>)}
      </div>
      <div className="min-h-0 flex-1 overflow-y-auto pr-1">
        {searchQuery.trim() ? <div className="grid gap-2"><div className="flex justify-between text-xs text-muted-foreground"><span>{t("layout.searchResults")}</span><span>{searchResults.length}</span></div>{searchResults.map((result) => <div className="grid gap-1" key={result.session.id}>{card(result.session)}<p className="truncate px-2 text-xs text-muted-foreground">{result.matches[0]?.excerpt}</p></div>)}{searchResults.length === 0 ? <p className="ucd-muted-panel rounded-md p-3 text-xs text-muted-foreground">{t("layout.noSearchResults")}</p> : null}</div> : null}
        {!searchQuery.trim() && mode !== "archived" && pinned.length > 0 ? <section className="mb-3 grid gap-2 border-b border-border pb-3"><div className="flex justify-between text-xs text-muted-foreground"><span><Pin className="mr-1 inline h-3.5 w-3.5" />{t("layout.pinned")}</span><span>{pinned.length}</span></div>{pinned.map(card)}</section> : null}
        {!searchQuery.trim() && mode === "activity" ? <div className="grid gap-3">{activity.map((group) => <section className="grid gap-2" key={group.key}><div className="flex justify-between text-xs text-muted-foreground"><span>{t(group.labelKey)}</span><span>{group.sessions.length}</span></div>{group.sessions.map(card)}</section>)}</div> : null}
        {!searchQuery.trim() && mode === "group" ? <div className="grid gap-2">{folders.map(([folder, grouped]) => <section className="grid gap-2" key={folder}><button className="ucd-list-row flex h-8 items-center gap-2 rounded-md px-2 text-left text-xs" onClick={() => toggle(folder)} type="button">{expanded.has(folder) ? <ChevronDown className="h-3.5 w-3.5" /> : <ChevronRight className="h-3.5 w-3.5" />}<Folder className="h-3.5 w-3.5 text-primary" /><span className="truncate">{folder}</span><span className="ml-auto">{grouped.length}</span></button>{expanded.has(folder) ? grouped.map(card) : null}</section>)}</div> : null}
        {!searchQuery.trim() && mode === "category" ? <div className="grid gap-2">{categoryGroups.map((group) => <section className="grid gap-2" key={group.id ?? "uncategorized"} onDragOver={(event) => event.preventDefault()} onDrop={(event) => dropCategory(event, group.id)}><button className="ucd-list-row flex h-8 items-center gap-2 rounded-md px-2 text-left text-xs" onClick={() => toggle(`category:${group.id ?? "none"}`)} type="button">{expanded.has(`category:${group.id ?? "none"}`) ? <ChevronDown className="h-3.5 w-3.5" /> : <ChevronRight className="h-3.5 w-3.5" />}<Tags className="h-3.5 w-3.5 text-primary" /><span className="truncate">{group.label}</span><span className="ml-auto">{group.sessions.length}</span></button>{expanded.has(`category:${group.id ?? "none"}`) ? group.sessions.map(card) : null}</section>)}</div> : null}
        {!searchQuery.trim() && mode === "archived" ? <div className="grid gap-2"><div className="flex justify-between text-xs text-muted-foreground"><span><Archive className="mr-1 inline h-3.5 w-3.5" />{t("layout.archived")}</span><span>{archivedSessions.length}</span></div>{archivedSessions.map(card)}{archivedSessions.length === 0 ? <p className="ucd-muted-panel rounded-md p-3 text-xs text-muted-foreground">{t("layout.noArchived")}</p> : null}</div> : null}
      </div>
    </aside>
  );
}
