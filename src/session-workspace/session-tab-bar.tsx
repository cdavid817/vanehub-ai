import { useRef, type KeyboardEvent } from "react";
import {
  Bot,
  FileDiff,
  FileText,
  Files,
  ScrollText,
  Shell,
  TerminalSquare,
  BarChart3,
  Activity,
  type LucideIcon,
} from "lucide-react";
import { useTranslation } from "react-i18next";
import { cn } from "../lib/utils";
import type { Session } from "../types/agent";
import { FolderOpenerControl } from "./folder-opener-control";
import { workspaceBadgeLabelKey, type WorkspaceTabBadge } from "./workspace-evidence-badges";

export type SessionTabId =
  | "chat"
  | "changes"
  | "documents"
  | "files"
  | "terminal"
  | "shell"
  | "logs"
  | "traces"
  | "report";

interface TabDefinition {
  id: SessionTabId;
  icon: LucideIcon;
}

export const sessionTabDefinitions: TabDefinition[] = [
  { id: "chat", icon: Bot },
  { id: "changes", icon: FileDiff },
  { id: "documents", icon: FileText },
  { id: "files", icon: Files },
  { id: "terminal", icon: TerminalSquare },
  { id: "shell", icon: Shell },
  { id: "logs", icon: ScrollText },
  { id: "traces", icon: Activity },
  { id: "report", icon: BarChart3 },
];

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
        return (
          <button
            aria-controls={`session-tab-panel-${id}`}
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
    </div>
  );
}

/**
 * A badge states a count or states that it has none — never both, and never a zero standing in for
 * an answer nobody has.
 *
 * The placeholder is a glyph with a spoken name. Rendering `0` for an index that is still building
 * would be a claim the workspace cannot support, and a reader has no way to tell the two apart.
 */
function TabBadge({ badge, tab }: { badge: WorkspaceTabBadge | undefined; tab: SessionTabId }) {
  const { t } = useTranslation();
  if (badge === undefined || badge.kind === "none") return null;
  const subject = t(workspaceBadgeLabelKey(tab));

  if (badge.kind === "unknown") {
    return (
      <span
        aria-label={t(`workspaceBadge.unknown.${badge.reason}`, { label: subject })}
        className="min-w-5 rounded-full border border-dashed border-border px-1 text-center font-mono text-[10px] text-muted-foreground"
        data-badge={`${tab}-unknown`}
        title={t(`workspaceBadge.unknown.${badge.reason}`, { label: subject })}
      >
        ·
      </span>
    );
  }

  const description = t(badge.atLeast ? "workspaceBadge.atLeast" : "workspaceBadge.count", {
    count: badge.count,
    label: subject,
  });
  return (
    <span
      aria-label={description}
      className={cn(
        "min-w-5 rounded-full border px-1 font-mono text-[10px]",
        badge.tone === "danger" ? "border-destructive text-destructive" : "border-border",
      )}
      data-badge={`${tab}-count`}
      title={description}
    >
      {badge.atLeast ? `≥${badge.count}` : badge.count}
    </span>
  );
}

