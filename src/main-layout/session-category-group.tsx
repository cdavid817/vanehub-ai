import type { DragEvent, ReactNode } from "react";
import { ChevronDown, ChevronRight, ListTree } from "lucide-react";
import { cn } from "../lib/utils";
import type { Session } from "../types/agent";
import type { SessionCategoryGroup as SessionCategoryGroupData } from "./session-sidebar-model";
import { SessionRowList } from "./session-row-list";

/**
 * 7.15: a visible drop target while dragging over it, and a brief success flash once the drop
 * lands — both purely presentational, neither an optimistic move. The row itself only reflects a
 * new category once the caller's assignment mutation actually succeeds, so a rejected drop has
 * nothing to visually roll back; its failure surfaces through that mutation's own error toast
 * instead of failing silently (see use-main-layout-model.ts's `assignCategory.onError`).
 */
export function SessionCategoryGroup({ batchMode, card, dragOverGroupKey, expanded, group, justDroppedGroupKey, onDrop, onToggle, setDragOverGroupKey }: {
  batchMode: boolean;
  card: (session: Session) => ReactNode;
  dragOverGroupKey: string | null;
  expanded: boolean;
  group: SessionCategoryGroupData;
  justDroppedGroupKey: string | null;
  onDrop: (event: DragEvent<HTMLElement>, categoryId: string | null, groupKey: string) => void;
  onToggle: () => void;
  setDragOverGroupKey: (groupKey: string | null) => void;
}) {
  const groupKey = group.id ?? "uncategorized";
  return (
    <section
      className={cn(
        "grid gap-2 rounded-md transition-colors",
        dragOverGroupKey === groupKey && "bg-[hsl(var(--nav-active-soft))] ring-1 ring-primary/50",
        justDroppedGroupKey === groupKey && "bg-[hsl(var(--success))]/10",
      )}
      data-session-category-id={groupKey}
      onDragEnter={() => { if (!batchMode) setDragOverGroupKey(groupKey); }}
      onDragLeave={(event) => { if (event.currentTarget === event.target) setDragOverGroupKey(null); }}
      onDragOver={(event) => { if (!batchMode) event.preventDefault(); }}
      onDrop={(event) => onDrop(event, group.id, groupKey)}
    >
      <button className="ucd-list-row flex h-8 items-center gap-2 rounded-md px-2 text-left text-xs" onClick={onToggle} type="button">
        {expanded ? <ChevronDown className="h-3.5 w-3.5" /> : <ChevronRight className="h-3.5 w-3.5" />}
        <ListTree className="h-3.5 w-3.5 text-primary" />
        <span className="truncate">{group.label}</span>
        <span className="ml-auto">{group.sessions.length}</span>
      </button>
      {expanded ? <SessionRowList ariaLabel={group.label} card={card} sessions={group.sessions} /> : null}
    </section>
  );
}
