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
  badges: Partial<Record<SessionTabId, number>>;
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
      className="ucd-segmented flex min-w-0 flex-1 gap-1 overflow-x-auto rounded-md p-1"
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
              "flex h-8 shrink-0 items-center gap-1.5 rounded-md px-2 text-xs transition-colors focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring",
              activeTab === id
                ? "bg-background font-semibold text-primary shadow-sm"
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
            {badge !== undefined ? (
              <span className="min-w-5 rounded-full border border-border px-1 font-mono text-[10px]" title={t("sessionTabs.badge", { count: badge })}>
                {badge}
              </span>
            ) : null}
          </button>
        );
      })}
    </div>
    <FolderOpenerControl onOpenSettings={onOpenSettings} session={session} />
    </div>
  );
}

