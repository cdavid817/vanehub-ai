import { useEffect, useRef, useState, type ReactNode } from "react";
import { Check, MoreHorizontal, PanelLeft, PanelRight, Rows3 } from "lucide-react";
import { useTranslation } from "react-i18next";
import { useMenuList } from "../ui/actions/use-menu-list";

interface VisibilityItem {
  expanded: boolean;
  icon: ReactNode;
  label: string;
  onToggle: () => void;
  testId: string;
}

export function ConversationOverflowMenu({
  infoPanelExpanded,
  onToggleInfoPanel,
  onToggleSessionList,
  onToggleWorkspaceTabs,
  sessionListExpanded,
  workspaceTabsExpanded,
}: {
  infoPanelExpanded: boolean;
  onToggleInfoPanel: () => void;
  onToggleSessionList: () => void;
  onToggleWorkspaceTabs: () => void;
  sessionListExpanded: boolean;
  workspaceTabsExpanded: boolean;
}) {
  const { t } = useTranslation();
  const [open, setOpen] = useState(false);
  const rootRef = useRef<HTMLDivElement>(null);
  const menuRef = useRef<HTMLDivElement>(null);
  const items: VisibilityItem[] = [
    { expanded: sessionListExpanded, icon: <PanelLeft aria-hidden="true" className="h-4 w-4" />, label: t("layout.conversationMenu.sessionList"), onToggle: onToggleSessionList, testId: "toggle-session-list" },
    { expanded: infoPanelExpanded, icon: <PanelRight aria-hidden="true" className="h-4 w-4" />, label: t("layout.conversationMenu.infoPanel"), onToggle: onToggleInfoPanel, testId: "toggle-info-panel" },
    { expanded: workspaceTabsExpanded, icon: <Rows3 aria-hidden="true" className="h-4 w-4" />, label: t("layout.conversationMenu.workspaceTabs"), onToggle: onToggleWorkspaceTabs, testId: "toggle-workspace-tabs" },
  ];
  const { activeIndex, handleMenuKeyDown, setActiveIndex } = useMenuList(items);
  const itemRefs = useRef<(HTMLButtonElement | null)[]>([]);

  useEffect(() => {
    if (!open) return;
    // Resets the roving index every time the menu opens -- this component (unlike a fresh popup
    // mount) persists across opens, so without this a stale index from a previous session would
    // carry over instead of starting back at the first item.
    setActiveIndex(0);
    const dismiss = (event: PointerEvent) => {
      if (!rootRef.current?.contains(event.target as Node)) setOpen(false);
    };
    const closeOnEscape = (event: KeyboardEvent) => {
      if (event.key === "Escape") setOpen(false);
    };
    document.addEventListener("pointerdown", dismiss);
    document.addEventListener("keydown", closeOnEscape);
    return () => {
      document.removeEventListener("pointerdown", dismiss);
      document.removeEventListener("keydown", closeOnEscape);
    };
  }, [open, setActiveIndex]);

  // Follows the roving index while open: fires once on open (focusing the first item) and again
  // on every Arrow/Home/End press (moving real DOM focus along with it).
  useEffect(() => {
    if (open) itemRefs.current[activeIndex]?.focus();
  }, [open, activeIndex]);

  return (
    <div className="relative" ref={rootRef}>
      <button
        aria-expanded={open}
        aria-haspopup="menu"
        aria-label={t("layout.conversationMenu")}
        className="grid h-9 w-9 place-items-center rounded-md text-muted-foreground transition-colors hover:bg-muted hover:text-foreground focus-visible:outline-hidden focus-visible:ring-2 focus-visible:ring-ring"
        data-testid="conversation-overflow-trigger"
        onClick={() => setOpen((value) => !value)}
        title={t("layout.conversationMenu")}
        type="button"
      >
        <MoreHorizontal aria-hidden="true" className="h-5 w-5" />
      </button>
      {open ? (
        <div className="absolute right-0 top-full z-50 mt-1 w-52 rounded-md border border-border bg-[hsl(var(--panel))] p-1 shadow-lg" onKeyDown={handleMenuKeyDown} ref={menuRef} role="menu">
          {items.map((item, index) => (
            <button
              aria-checked={item.expanded}
              className="flex h-9 w-full items-center gap-2 rounded px-2 text-left text-sm text-foreground hover:bg-muted focus-visible:outline-hidden focus-visible:ring-2 focus-visible:ring-ring"
              data-testid={item.testId}
              key={item.testId}
              onClick={() => {
                item.onToggle();
                setOpen(false);
              }}
              onFocus={() => setActiveIndex(index)}
              ref={(element) => { itemRefs.current[index] = element; }}
              role="menuitemcheckbox"
              tabIndex={index === activeIndex ? 0 : -1}
              type="button"
            >
              <span className="text-muted-foreground">{item.icon}</span>
              <span className="min-w-0 flex-1 truncate">{item.label}</span>
              <Check aria-hidden="true" className={item.expanded ? "h-4 w-4 text-primary" : "h-4 w-4 opacity-0"} />
            </button>
          ))}
        </div>
      ) : null}
    </div>
  );
}
