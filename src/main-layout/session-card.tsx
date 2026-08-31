import type { ChangeEvent, DragEvent, MouseEvent } from "react";
import { EllipsisVertical, Pin, UsersRound } from "lucide-react";
import { useTranslation } from "react-i18next";
import { AgentBrandIcon } from "../components/agent-brand-icon";
import { getAgentVisualIdentity } from "../lib/agent-visual-identity";
import { lifecycleDotClass, lifecycleLabelKey, lifecycleTone } from "../lib/session-lifecycle";
import { cn } from "../lib/utils";
import type { Session } from "../types/agent";

export function SessionCard({ active, batchMode, checked, draggable, onContextMenu, onDragStart, onOpenActions, onSelect, onToggleChecked, session }: {
  active: boolean; batchMode: boolean; checked: boolean; draggable?: boolean;
  onContextMenu: (event: MouseEvent<HTMLButtonElement>) => void;
  onDragStart?: (event: DragEvent<HTMLDivElement>) => void;
  onOpenActions: (event: MouseEvent<HTMLButtonElement>) => void;
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
  const needsReview = session.recoveryStatus === "action_required" || session.recoveryStatus === "quarantined";
  return (
    // 7.14: a `<button>` cannot contain another interactive `<button>`, so the trailing action
    // trigger has to be a sibling of the select button, not nested inside it — both live under
    // this one draggable/context-menu-able wrapper so 7.15's drag and right-click behavior is
    // unaffected by where inside the row the pointer lands.
    <div
      className="group relative w-full"
      data-session-id={session.id}
      draggable={draggable}
      onContextMenu={batchMode ? (event) => event.preventDefault() : undefined}
      onDragStart={onDragStart}
    >
      <button
        aria-pressed={batchMode ? checked : active}
        className={cn(
          "w-full rounded-md border border-transparent px-3 py-2.5 text-left transition-colors hover:bg-[hsl(var(--panel-hover))] focus-visible:outline-hidden focus-visible:ring-2 focus-visible:ring-inset focus-visible:ring-ring",
          active && !batchMode && "border-border/70 bg-background shadow-xs",
          checked && batchMode && "border-primary/50 bg-[hsl(var(--nav-active-soft))]",
        )}
        onClick={select}
        onContextMenu={batchMode ? undefined : onContextMenu}
        type="button"
      >
        {active && !batchMode ? <span className="absolute inset-y-2 left-0 w-0.5 rounded bg-primary" /> : null}
        <div className="flex min-w-0 items-center gap-2 pr-6">
          {batchMode ? <input aria-label={t("layout.batchSelectSession")} checked={checked} className="h-4 w-4 shrink-0 accent-[hsl(var(--primary))]" onChange={checkboxChanged} onClick={(event) => event.stopPropagation()} type="checkbox" /> : null}
          <span className={cn("flex h-9 w-9 shrink-0 items-center justify-center rounded-lg border", meta.tone)} title={meta.label}><AgentBrandIcon agentId={session.agentId} className="h-5 w-5" /></span>
          <span className={cn("min-w-0 flex-1 truncate text-sm font-medium", session.archived && "text-muted-foreground")}>{session.title}</span>
          {session.pinned ? <Pin aria-hidden="true" className="h-3.5 w-3.5 shrink-0 text-primary" /> : null}
        </div>
        <div className="mt-1 flex min-w-0 items-center gap-1.5 overflow-hidden pl-11 pr-6 text-[11px] text-muted-foreground">
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
          {/* 7.9: compact, icon-only, present-only-when-true — a fourth full badge here would
              blow the one-bounded-secondary-line budget (7.8) this row already spends on
              lifecycle + agent identity. */}
          {needsReview ? <span aria-hidden="true" data-testid="session-needs-review-indicator" title={t("layout.sessionNeedsReview")}>⚠</span> : null}
          {session.source?.kind === "im" ? <span aria-hidden="true" data-testid="session-im-indicator" title={t("layout.sessionSourceIm")}>IM</span> : null}
          {session.remoteWorkspace ? <span aria-hidden="true" data-testid="session-remote-indicator" title={t("layout.sessionRemote")}>SSH</span> : null}
          <span className="ml-auto shrink-0 font-mono">{date}</span>
        </div>
      </button>
      <button
        aria-label={t("layout.sessionRowActions")}
        className="absolute right-1 top-1.5 grid h-6 w-6 shrink-0 place-items-center rounded text-muted-foreground opacity-0 hover:bg-muted focus-visible:opacity-100 focus-visible:outline-hidden focus-visible:ring-2 focus-visible:ring-ring group-hover:opacity-100"
        onClick={onOpenActions}
        title={t("layout.sessionRowActions")}
        type="button"
      >
        <EllipsisVertical aria-hidden="true" className="h-3.5 w-3.5" />
      </button>
    </div>
  );
}
