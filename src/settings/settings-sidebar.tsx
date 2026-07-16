import { Button } from "../components/ui/button";
import { settingsPages, type SettingsPageId } from "./settings-pages";
import { useTranslation } from "react-i18next";

interface SettingsSidebarProps {
  activePageId: SettingsPageId;
  onSelectPage: (pageId: SettingsPageId) => void;
}

export function SettingsSidebar({ activePageId, onSelectPage }: SettingsSidebarProps) {
  const { t } = useTranslation();

  return (
    <aside className="ucd-panel flex min-h-0 flex-col rounded-lg p-2">
      <div className="px-3 py-4">
        <div className="text-xs font-bold uppercase tracking-[0.18em] text-muted-foreground">{t("app.settings.system")}</div>
      </div>
      <nav className="grid gap-1 border-t border-border pt-2">
        {settingsPages.map((page) => {
          const Icon = page.icon;
          const active = page.id === activePageId;
          return (
            <button
              className={`relative flex h-10 min-w-0 items-center gap-2 rounded-md px-3 text-left text-sm transition-colors ${
                active ? "bg-[hsl(var(--nav-active-soft))] font-semibold text-primary" : "text-foreground hover:bg-muted"
              }`}
              key={page.id}
              onClick={() => onSelectPage(page.id)}
              type="button"
            >
              {active ? <span className="absolute left-0 h-6 w-0.5 rounded bg-primary" /> : null}
              <Icon className="h-4 w-4 shrink-0" aria-hidden="true" />
              <span className="min-w-0 flex-1 truncate">{t(page.labelKey)}</span>
              {page.badge ? (
                <span className="inline-flex h-5 min-w-5 items-center justify-center rounded-full bg-[hsl(var(--nav-active-soft))] px-1.5 text-xs text-primary">
                  {page.badge}
                </span>
              ) : null}
            </button>
          );
        })}
      </nav>

      <div className="mt-auto grid gap-2 border-t border-border px-3 py-4 text-xs text-muted-foreground">
        <span>VaneHub AI v0.1.0</span>
        <span>Build 2026-07-14</span>
        <Button className="mt-2 justify-start" size="default" variant="outline">
          {t("app.settings.export")}
        </Button>
      </div>
    </aside>
  );
}
