import { Download } from "lucide-react";
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
    <aside className="ucd-panel flex min-h-0 flex-col rounded-lg p-2 max-lg:block max-lg:overflow-hidden">
      <div className="px-3 py-3 max-lg:hidden">
        <div className="text-xs font-bold uppercase tracking-[0.16em] text-muted-foreground">{t("app.settings.system")}</div>
      </div>
      <nav className="grid gap-1 border-t border-border pt-2 max-lg:flex max-lg:overflow-x-auto max-lg:border-t-0 max-lg:pt-0">
        {settingsPages.map((page) => {
          const Icon = page.icon;
          const active = page.id === activePageId;
          return (
            <button
              className={`relative flex h-10 min-w-0 items-center gap-2 rounded-md px-3 text-left text-sm transition-colors max-lg:shrink-0 ${
                active ? "bg-[hsl(var(--nav-active-soft))] font-semibold text-primary shadow-sm" : "text-foreground hover:bg-muted"
              }`}
              key={page.id}
              onClick={() => onSelectPage(page.id)}
              type="button"
            >
              {active ? <span className="absolute left-0 h-6 w-0.5 rounded bg-primary" /> : null}
              <span className="flex h-7 w-7 shrink-0 items-center justify-center rounded-xl border border-border bg-[hsl(var(--panel-muted))]">
                <Icon className="h-3.5 w-3.5" aria-hidden="true" />
              </span>
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

      <div className="mt-auto grid gap-2 border-t border-border px-3 py-4 text-xs text-muted-foreground max-lg:hidden">
        <span>VaneHub AI v0.1.0</span>
        <span>Build 2026-07-14</span>
        <Button className="mt-2 justify-start" size="sm" title={t("app.settings.export")} variant="outline">
          <Download className="h-4 w-4" aria-hidden="true" />
          {t("app.settings.export")}
        </Button>
      </div>
    </aside>
  );
}
