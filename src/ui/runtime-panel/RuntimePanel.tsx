import { useRef, type ReactNode } from "react";
import { Maximize2, Minimize2, X, type LucideIcon } from "lucide-react";
import { useTranslation } from "react-i18next";
import { cn } from "../../lib/utils";
import { useTabList } from "./use-tab-list";

export interface RuntimePanelTab {
  id: string;
  label: string;
  icon?: LucideIcon;
  badge?: ReactNode;
  /**
   * Receives the tab's current visibility so its own effects (polling, live subscriptions) can
   * pause themselves — design.md Decision 7: a hidden tab must not rely on `display:none` alone
   * to stop working, since that hides the DOM but does not pause anything running inside it.
   */
  render: (isVisible: boolean) => ReactNode;
}

export interface RuntimePanelProps {
  tabs: RuntimePanelTab[];
  activeTabId: string;
  onActiveTabChange: (tabId: string) => void;
  onClose: () => void;
  maximized: boolean;
  onMaximizedChange: (maximized: boolean) => void;
  className?: string;
}

/**
 * Only mounts a tab once it has actually been opened, and keeps it mounted afterward rather than
 * unmounting on every switch — a terminal or shell tab losing its process on every tab change
 * would defeat the retention model in design.md Decision 7. Resize itself is not this shell's
 * concern: it lives in the vertical `SplitPane` gutter that hosts this panel in
 * `DestinationLayout`; "maximize" here is only the state toggle, since giving the panel full
 * height is a composition decision for whoever assembles the layout, not something this shell
 * can force from inside its own container.
 */
export function RuntimePanel({ tabs, activeTabId, onActiveTabChange, onClose, maximized, onMaximizedChange, className }: RuntimePanelProps) {
  const { t } = useTranslation();
  const everOpenedRef = useRef<Set<string>>(new Set());
  if (!everOpenedRef.current.has(activeTabId)) {
    everOpenedRef.current = new Set(everOpenedRef.current).add(activeTabId);
  }
  const { handleKeyDown, registerTabRef } = useTabList(tabs, activeTabId, onActiveTabChange);

  return (
    <div className={cn("flex h-full min-h-0 flex-col", className)}>
      <div className="flex shrink-0 items-center justify-between border-b border-border-subtle">
        <div className="flex items-center gap-1 overflow-x-auto px-2" onKeyDown={handleKeyDown} role="tablist">
          {tabs.map((tab) => (
            <button
              aria-selected={tab.id === activeTabId}
              className={cn(
                "flex items-center gap-1.5 whitespace-nowrap border-b-2 px-2.5 py-2 text-sm",
                tab.id === activeTabId ? "border-primary text-foreground" : "border-transparent text-muted-foreground hover:text-foreground",
              )}
              id={`runtime-panel-tab-${tab.id}`}
              key={tab.id}
              onClick={() => onActiveTabChange(tab.id)}
              ref={registerTabRef(tab.id)}
              role="tab"
              tabIndex={tab.id === activeTabId ? 0 : -1}
              type="button"
            >
              {tab.icon ? <tab.icon aria-hidden="true" className="h-3.5 w-3.5" /> : null}
              {tab.label}
              {tab.badge}
            </button>
          ))}
        </div>
        <div className="flex shrink-0 items-center gap-1 px-2">
          <button
            aria-label={t(maximized ? "workbenchUi.runtimePanel.restore" : "workbenchUi.runtimePanel.maximize")}
            className="ucd-focus-ring rounded-md p-1.5 hover:bg-accent"
            onClick={() => onMaximizedChange(!maximized)}
            type="button"
          >
            {maximized ? <Minimize2 aria-hidden="true" className="h-3.5 w-3.5" /> : <Maximize2 aria-hidden="true" className="h-3.5 w-3.5" />}
          </button>
          <button aria-label={t("workbenchUi.inspector.close")} className="ucd-focus-ring rounded-md p-1.5 hover:bg-accent" onClick={onClose} type="button">
            <X aria-hidden="true" className="h-3.5 w-3.5" />
          </button>
        </div>
      </div>
      <div className="min-h-0 flex-1 overflow-y-auto">
        {tabs.filter((tab) => everOpenedRef.current.has(tab.id)).map((tab) => {
          const isVisible = tab.id === activeTabId;
          return (
            <div aria-labelledby={`runtime-panel-tab-${tab.id}`} hidden={!isVisible} key={tab.id} role="tabpanel">
              {tab.render(isVisible)}
            </div>
          );
        })}
      </div>
    </div>
  );
}
