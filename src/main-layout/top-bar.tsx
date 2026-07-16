import { Bell, Search } from "lucide-react";
import { useTranslation } from "react-i18next";

export function TopBar() {
  const { t } = useTranslation();

  return (
    <header className="ucd-panel mx-2 mt-2 flex min-h-12 items-center justify-between gap-3 rounded-xl px-4 py-2">
      <div className="flex min-w-0 items-center gap-3">
        <div className="flex h-7 w-7 shrink-0 items-center justify-center rounded-md border border-primary bg-[hsl(var(--nav-active-soft))] text-sm font-bold text-primary">
          V
        </div>
        <div className="min-w-0">
          <div className="flex items-center gap-3">
            <h1 className="truncate text-base font-bold">VaneHub AI</h1>
            <span className="hidden font-mono text-xs text-muted-foreground sm:inline">#SID-20260714</span>
          </div>
        </div>
      </div>

      <div className="hidden min-w-72 max-w-sm flex-1 lg:block">
        <div className="relative">
          <Search className="pointer-events-none absolute left-3 top-1/2 h-4 w-4 -translate-y-1/2 text-muted-foreground" />
          <input
            className="ucd-input h-8 w-full rounded-md px-9 text-sm outline-none focus-visible:ring-2 focus-visible:ring-ring"
            placeholder={t("layout.searchPlaceholder")}
          />
        </div>
      </div>

      <div className="flex items-center gap-2">
        <button className="relative flex h-8 w-9 items-center justify-center rounded border border-border hover:bg-muted" type="button">
          <Bell className="h-4 w-4 text-muted-foreground" aria-hidden="true" />
          <span className="absolute right-2 top-1.5 h-2 w-2 rounded-full bg-[hsl(var(--danger))]" />
        </button>
      </div>
    </header>
  );
}
