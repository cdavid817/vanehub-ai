import { useTranslation } from "react-i18next";
import { aboutCurrentVersion } from "../services/about-service";
import {
  settingsPageGroupOrder,
  settingsPages,
  type SettingsPageGroup,
  type SettingsPageId,
} from "./settings-pages";

interface SettingsSidebarProps {
  activePageId: SettingsPageId;
  onSelectPage: (pageId: SettingsPageId) => void;
}

const groupLabelKeys: Record<SettingsPageGroup, string> = {
  general: "settings.group.general",
  agent: "settings.group.agent",
  capabilities: "settings.group.capabilities",
  integrations: "settings.group.integrations",
  diagnostics: "settings.group.diagnostics",
};

export function SettingsSidebar({ activePageId, onSelectPage }: SettingsSidebarProps) {
  const { t } = useTranslation();

  return (
    <aside className="flex min-h-0 flex-col rounded-lg border border-border bg-background p-2 shadow-xs max-lg:block max-lg:overflow-hidden">
      {/* grid-cols-[minmax(0,1fr)] below is load-bearing: an auto grid column takes its minimum
          from content, so a nowrap label longer than the sidebar widened the column past the
          scroll container and clipped the selected entry's highlight on both edges. */}
      <nav
        aria-label={t("app.settings.system")}
        className="grid min-h-0 flex-1 grid-cols-[minmax(0,1fr)] gap-1 overflow-y-auto pt-1 max-lg:flex max-lg:grid-cols-none max-lg:overflow-x-auto max-lg:overflow-y-hidden max-lg:pt-0"
      >
        {settingsPageGroupOrder.map((group) => {
          const pages = settingsPages.filter((page) => page.group === group);
          if (pages.length === 0) return null;
          return (
            <div className="grid min-w-0 grid-cols-[minmax(0,1fr)] gap-1 max-lg:flex max-lg:shrink-0 max-lg:grid-cols-none max-lg:gap-1" key={group}>
              <div className="px-2.5 pb-0.5 pt-2 text-[11px] font-semibold tracking-wide text-muted-foreground max-lg:hidden">
                {t(groupLabelKeys[group])}
              </div>
              {pages.map((page) => {
                const Icon = page.icon;
                const active = page.id === activePageId;
                return (
                  <button
                    className={`relative flex min-h-10 min-w-0 items-center gap-2 rounded-md px-2.5 py-2 text-left text-sm transition-colors max-lg:min-w-max max-lg:shrink-0 ${
                      active ? "bg-[hsl(var(--nav-active-soft))] font-semibold text-primary" : "text-foreground hover:bg-muted"
                    }`}
                    key={page.id}
                    onClick={() => onSelectPage(page.id)}
                    type="button"
                  >
                    {active ? <span className="absolute left-0 h-5 w-0.5 rounded bg-primary max-lg:hidden" /> : null}
                    <span className="flex h-7 w-7 shrink-0 items-center justify-center rounded-md border border-border bg-[hsl(var(--panel-muted))]">
                      <Icon className="h-3.5 w-3.5" aria-hidden="true" />
                    </span>
                    <span className="min-w-0 flex-1 truncate" title={t(page.labelKey)}>{t(page.labelKey)}</span>
                    {page.badge ? (
                      <span className="inline-flex h-5 min-w-5 shrink-0 items-center justify-center rounded-full bg-[hsl(var(--nav-active-soft))] px-1.5 text-xs text-primary">
                        {page.badge}
                      </span>
                    ) : null}
                  </button>
                );
              })}
            </div>
          );
        })}
      </nav>

      <div className="mt-auto border-t border-border px-3 py-3 text-xs leading-5 text-muted-foreground max-lg:hidden">
        VaneHub AI v{aboutCurrentVersion}
      </div>
    </aside>
  );
}
