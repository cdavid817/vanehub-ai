import { Search, X } from "lucide-react";
import { useState } from "react";
import { useTranslation } from "react-i18next";
import { NotificationCenter } from "../notifications/notification-center";
import { ConversationFocusButton } from "./conversation-focus-button";

export function TopBar({ focusMode, focusModeAvailable, onFocusModeChange }: {
  focusMode: boolean;
  focusModeAvailable: boolean;
  onFocusModeChange: (active: boolean) => void;
}) {
  const { t } = useTranslation();
  const [searchOpen, setSearchOpen] = useState(false);

  function toggleFocusMode() {
    if (!focusMode) setSearchOpen(false);
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

  if (searchOpen) {
    return (
      <header className="relative z-40 flex h-10 items-center gap-2 border-b border-border/70 bg-[hsl(var(--panel))] px-3" data-focus-collapsed="false" data-testid="top-bar">
        <div className="relative min-w-0 flex-1">
          <Search aria-hidden="true" className="pointer-events-none absolute left-3 top-1/2 h-4 w-4 -translate-y-1/2 text-muted-foreground" />
          <input
            autoFocus
            className="ucd-input h-8 w-full rounded-md px-9 text-sm outline-hidden focus-visible:ring-2 focus-visible:ring-ring"
            placeholder={t("layout.searchPlaceholder")}
          />
        </div>
        {focusModeAvailable ? <ConversationFocusButton active={focusMode} onToggle={toggleFocusMode} /> : null}
        <button
          aria-label={t("layout.closeSearch")}
          className="grid h-8 w-8 shrink-0 place-items-center rounded-md text-muted-foreground hover:bg-muted hover:text-foreground focus-visible:outline-hidden focus-visible:ring-2 focus-visible:ring-ring"
          onClick={() => setSearchOpen(false)}
          title={t("layout.closeSearch")}
          type="button"
        >
          <X aria-hidden="true" className="h-4 w-4" />
        </button>
      </header>
    );
  }

  return (
    <header className="relative z-40 flex h-10 items-center justify-between gap-3 border-b border-border/70 bg-[hsl(var(--panel))] px-3" data-focus-collapsed="false" data-testid="top-bar">
      <div className="flex min-w-0 items-center gap-3">
        <div className="flex h-6 w-6 shrink-0 items-center justify-center rounded-md border border-primary/60 bg-[hsl(var(--nav-active-soft))] text-xs font-bold text-primary">
          V
        </div>
        <div className="min-w-0">
          <div className="flex items-center gap-3">
            <h1 className="truncate text-sm font-semibold">VaneHub AI</h1>
            <span className="hidden font-mono text-[11px] text-muted-foreground sm:inline">#SID-20260714</span>
          </div>
        </div>
      </div>

      <div className="flex items-center gap-2">
        {focusModeAvailable ? <ConversationFocusButton active={focusMode} onToggle={toggleFocusMode} /> : null}
        <button
          aria-label={t("layout.openSearch")}
          className="grid h-8 w-8 shrink-0 place-items-center rounded-md text-muted-foreground hover:bg-muted hover:text-foreground focus-visible:outline-hidden focus-visible:ring-2 focus-visible:ring-ring"
          onClick={() => setSearchOpen(true)}
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
