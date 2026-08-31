import { useRef, type KeyboardEvent } from "react";
import { Bot, FileDiff, Files, BarChart3, type LucideIcon } from "lucide-react";
import { useTranslation } from "react-i18next";
import { cn } from "../lib/utils";
import type { Session } from "../types/agent";
import type { SessionPrimarySurfaceId } from "./session-surface-registry";
import { FolderOpenerControl } from "./folder-opener-control";
import { workspaceBadgeLabelKey, type WorkspaceTabBadge } from "./workspace-evidence-badges";

export type SessionTabId = SessionPrimarySurfaceId;

interface TabDefinition {
  id: SessionTabId;
  icon: LucideIcon;
}

/**
 * Just the primary four — Terminal History, Shell, Logs, and Traces moved to the Runtime Panel
 * (`session-runtime-panel.tsx`) and render their own tab strip there (`src/ui/runtime-panel`).
 */
export const sessionTabDefinitions: TabDefinition[] = [
  { id: "work", icon: Bot },
  { id: "changes", icon: FileDiff },
  { id: "files", icon: Files },
  { id: "report", icon: BarChart3 },
];

function badgeDescriptionId(tab: SessionTabId): string {
  return `session-tab-badge-${tab}`;
}

/**
 * What a badge says out loud: the subject it counts and the count, or why there is no count.
 *
 * Returns null for a tab with nothing to report, which is how the button decides whether it has a
 * description at all.
 */
function badgeDescription(
  badge: WorkspaceTabBadge | undefined,
  tab: SessionTabId,
  t: (key: string, values?: Record<string, string | number>) => string,
): string | null {
  if (badge === undefined || badge.kind === "none") return null;
  const label = t(workspaceBadgeLabelKey(tab));
  if (badge.kind === "unknown") return t(`workspaceBadge.unknown.${badge.reason}`, { label });
  return t(badge.atLeast ? "workspaceBadge.atLeast" : "workspaceBadge.count", {
    count: badge.count,
    label,
  });
}

export function SessionTabBar({
  activeTab,
  badges,
  onActivate,
  session,
  onOpenSettings,
}: {
  activeTab: SessionTabId;
  badges: Partial<Record<SessionTabId, WorkspaceTabBadge>>;
  onActivate: (tab: SessionTabId) => void;
  session: Session | null;
  onOpenSettings: () => void;
}) {
  const { t } = useTranslation();
  const buttonRefs = useRef<Array<HTMLButtonElement | null>>([]);
  const descriptions = sessionTabDefinitions.flatMap(({ id }) => {
    const text = badgeDescription(badges[id], id, t);
    return text === null ? [] : [{ id, text }];
  });

  function handleKeyDown(event: KeyboardEvent<HTMLButtonElement>, index: number) {
    let nextIndex: number | null = null;
    if (event.key === "ArrowRight") nextIndex = (index + 1) % sessionTabDefinitions.length;
    if (event.key === "ArrowLeft") nextIndex = (index - 1 + sessionTabDefinitions.length) % sessionTabDefinitions.length;
    if (event.key === "Home") nextIndex = 0;
    if (event.key === "End") nextIndex = sessionTabDefinitions.length - 1;
    if (nextIndex === null) return;
    event.preventDefault();
    const nextTab = sessionTabDefinitions[nextIndex];
    onActivate(nextTab.id);
    buttonRefs.current[nextIndex]?.focus();
  }

  return (
    <div className="flex min-w-0 shrink-0 items-center gap-2">
    <div
      aria-label={t("sessionTabs.ariaLabel")}
      className="ucd-segmented ucd-scroll-strip flex min-w-0 flex-1 gap-1 overflow-x-auto rounded-md p-1"
      role="tablist"
    >
      {sessionTabDefinitions.map(({ id, icon: Icon }, index) => {
        const label = t(`sessionTabs.tab.${id}`);
        const badge = badges[id];
        const described = descriptions.some((entry) => entry.id === id);
        return (
          <button
            aria-controls={`session-tab-panel-${id}`}
            // Described, not renamed. A tab's accessible name identifies the tab; folding a live
            // count into it makes the name change as work runs, and a name like "Changes,
            // unviewed changed files: 4" then matches a search for the Files tab.
            aria-describedby={described ? badgeDescriptionId(id) : undefined}
            aria-selected={activeTab === id}
            className={cn(
              "flex h-8 shrink-0 items-center gap-1.5 rounded-md px-2 text-xs transition-colors focus-visible:outline-hidden focus-visible:ring-2 focus-visible:ring-ring",
              activeTab === id
                ? "bg-background font-semibold text-primary shadow-xs"
                : "text-muted-foreground hover:bg-muted hover:text-foreground",
            )}
            id={`session-tab-${id}`}
            key={id}
            onClick={() => onActivate(id)}
            onKeyDown={(event) => handleKeyDown(event, index)}
            ref={(element) => {
              buttonRefs.current[index] = element;
            }}
            role="tab"
            tabIndex={activeTab === id ? 0 : -1}
            title={label}
            type="button"
          >
            <Icon aria-hidden="true" className="h-3.5 w-3.5" />
            <span>{label}</span>
            <TabBadge badge={badge} tab={id} />
          </button>
        );
      })}
    </div>
    <FolderOpenerControl onOpenSettings={onOpenSettings} session={session} />
    {/* Outside the buttons on purpose: a description rendered inside one would be read as part of
        that button's name, which is the thing `aria-describedby` exists to avoid. */}
    <div className="sr-only">
      {descriptions.map((entry) => (
        <span id={badgeDescriptionId(entry.id)} key={entry.id}>
          {entry.text}
        </span>
      ))}
    </div>
    </div>
  );
}

/**
 * The visible half of a badge: a count, a floor, or a placeholder glyph.
 *
 * Hidden from the accessibility tree because the button carries the spoken version as its
 * description. A `0` never appears here — rendering one for an index that has not finished
 * building would be a claim the workspace cannot support, and a reader cannot tell the two apart.
 */
function TabBadge({ badge, tab }: { badge: WorkspaceTabBadge | undefined; tab: SessionTabId }) {
  if (badge === undefined || badge.kind === "none") return null;

  if (badge.kind === "unknown") {
    return (
      <span
        aria-hidden="true"
        className="min-w-5 rounded-full border border-dashed border-border px-1 text-center font-mono text-[10px] text-muted-foreground"
        data-badge={`${tab}-unknown`}
      >
        ·
      </span>
    );
  }

  return (
    <span
      aria-hidden="true"
      className={cn(
        "min-w-5 rounded-full border px-1 font-mono text-[10px]",
        badge.tone === "danger" ? "border-destructive text-destructive" : "border-border",
      )}
      data-badge={`${tab}-count`}
    >
      {badge.atLeast ? `≥${badge.count}` : badge.count}
    </span>
  );
}
