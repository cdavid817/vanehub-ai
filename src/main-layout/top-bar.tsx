import { Search } from "lucide-react";
import { useTranslation } from "react-i18next";
import { NotificationCenter } from "../notifications/notification-center";
import { ConversationFocusButton } from "./conversation-focus-button";

export function TopBar({ focusMode, focusModeAvailable, onFocusModeChange, onSearch }: {
  focusMode: boolean;
  focusModeAvailable: boolean;
  onFocusModeChange: (active: boolean) => void;
  onSearch: () => void;
}) {
  const { t } = useTranslation();

  function toggleFocusMode() {
    onFocusModeChange(!focusMode);
  }

  if (focusMode && focusModeAvailable) {
    return (
      <header
        className="relative z-40 flex h-9 shrink-0 items-center justify-between border-b border-border/70 bg-[hsl(var(--panel))] px-2"
        data-focus-collapsed="true"
        data-testid="top-bar"
      >
        <div className="flex h-6 w-6 items-center justify-center rounded-md border border-primary/40 bg-[hsl(var(--nav-active-soft))] text-xs font-bold text-primary">
          V
        </div>
        <ConversationFocusButton active labelVisible onToggle={toggleFocusMode} />
      </header>
    );
  }

  return (
    <header className="relative z-40 flex h-10 items-center justify-between gap-3 border-b border-border/70 bg-[hsl(var(--panel))] px-3" data-focus-collapsed="false" data-testid="top-bar">
      <div className="flex min-w-0 items-center gap-3">
        <div className="flex h-6 w-6 shrink-0 items-center justify-center rounded-md border border-primary/60 bg-[hsl(var(--nav-active-soft))] text-xs font-bold text-primary">
          V
        </div>
        <h1 className="truncate text-sm font-semibold">VaneHub AI</h1>
      </div>

      <div className="flex items-center gap-2">
        {focusModeAvailable ? <ConversationFocusButton active={focusMode} onToggle={toggleFocusMode} /> : null}
        {/* This used to open a second input that had no value binding and no submit path. It now
            reveals and focuses the session search that actually runs a query. */}
        <button
          aria-controls="workspace-session-search"
          aria-label={t("layout.openSearch")}
          className="grid h-8 w-8 shrink-0 place-items-center rounded-md text-muted-foreground hover:bg-muted hover:text-foreground focus-visible:outline-hidden focus-visible:ring-2 focus-visible:ring-ring"
          onClick={onSearch}
          title={t("layout.openSearch")}
          type="button"
        >
          <Search aria-hidden="true" className="h-4 w-4" />
        </button>
        <NotificationCenter />
      </div>
    </header>
  );
}
