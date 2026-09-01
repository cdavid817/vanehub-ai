import { useState } from "react";
import { ChevronDown, Search } from "lucide-react";
import { useTranslation } from "react-i18next";
import { Sheet } from "../ui/sheet/Sheet";
import { getSettingsPage, settingsPageGroupOrder, settingsPages, type SettingsPageGroup, type SettingsPageId } from "./settings-pages";

const groupLabelKeys: Record<SettingsPageGroup, string> = {
  general: "settings.group.general",
  agent: "settings.group.agent",
  capabilities: "settings.group.capabilities",
  integrations: "settings.group.integrations",
  diagnostics: "settings.group.diagnostics",
};

/**
 * Task 12.9's compact-width replacement for the old horizontal page strip: a trigger naming the
 * current page, opening a searchable Sheet listing every page grouped the same way the desktop
 * sidebar does -- spec.md's "become a searchable sheet or selector... reachable without
 * horizontal scrolling through every page". Rendered alongside `SettingsSidebar` in the DOM at
 * all widths and toggled via the same `max-lg:`/`lg:hidden` CSS-breakpoint convention that
 * component already uses, not a `useMediaQuery` runtime switch, to match its existing idiom.
 */
export function SettingsCompactNav({ activePageId, onSelectPage }: { activePageId: SettingsPageId; onSelectPage: (pageId: SettingsPageId) => void }) {
  const { t } = useTranslation();
  const [open, setOpen] = useState(false);
  const [filter, setFilter] = useState("");
  const activePage = getSettingsPage(activePageId);
  const normalizedFilter = filter.trim().toLowerCase();

  function selectPage(pageId: SettingsPageId) {
    onSelectPage(pageId);
    setOpen(false);
    setFilter("");
  }

  return (
    <div className="lg:hidden">
      <button
        aria-expanded={open}
        aria-haspopup="dialog"
        aria-label={t("settings.compactNav.trigger", { page: t(activePage.labelKey) })}
        className="ucd-list-row flex h-10 w-full items-center gap-2 rounded-md border border-border bg-background px-3 text-left text-sm"
        onClick={() => setOpen(true)}
        type="button"
      >
        <span className="flex h-6 w-6 shrink-0 items-center justify-center rounded border border-border bg-[hsl(var(--panel-muted))]">
          <activePage.icon className="h-3.5 w-3.5" aria-hidden="true" />
        </span>
        <span className="min-w-0 flex-1 truncate font-medium">{t(activePage.labelKey)}</span>
        <ChevronDown aria-hidden="true" className="h-4 w-4 shrink-0 text-muted-foreground" />
      </button>
      {open ? (
        <Sheet
          className="max-w-sm"
          closeDisabled={false}
          onClose={() => { setOpen(false); setFilter(""); }}
          placement="left"
          title={t("app.settings.system")}
        >
          <div className="grid gap-3">
            <div className="relative">
              <Search className="pointer-events-none absolute left-3 top-1/2 h-4 w-4 -translate-y-1/2 text-muted-foreground" />
              <input
                autoFocus
                className="ucd-input h-9 w-full rounded-md px-9 text-sm outline-hidden focus-visible:ring-2 focus-visible:ring-ring"
                onChange={(event) => setFilter(event.target.value)}
                placeholder={t("settings.compactNav.filterPlaceholder")}
                value={filter}
              />
            </div>
            {settingsPageGroupOrder.map((group) => {
              const pages = settingsPages.filter(
                (page) => page.group === group && (!normalizedFilter || t(page.labelKey).toLowerCase().includes(normalizedFilter)),
              );
              if (pages.length === 0) return null;
              return (
                <div className="grid gap-1" key={group}>
                  <div className="px-1 text-[11px] font-semibold tracking-wide text-muted-foreground">{t(groupLabelKeys[group])}</div>
                  {pages.map((page) => {
                    const Icon = page.icon;
                    const active = page.id === activePageId;
                    return (
                      <button
                        aria-current={active ? "page" : undefined}
                        className={`flex min-h-10 items-center gap-2 rounded-md px-2.5 py-2 text-left text-sm ${active ? "bg-[hsl(var(--nav-active-soft))] font-semibold text-primary" : "text-foreground hover:bg-muted"}`}
                        key={page.id}
                        onClick={() => selectPage(page.id)}
                        type="button"
                      >
                        <span className="flex h-7 w-7 shrink-0 items-center justify-center rounded-md border border-border bg-[hsl(var(--panel-muted))]">
                          <Icon className="h-3.5 w-3.5" aria-hidden="true" />
                        </span>
                        <span className="min-w-0 flex-1 truncate">{t(page.labelKey)}</span>
                      </button>
                    );
                  })}
                </div>
              );
            })}
            {normalizedFilter && settingsPages.every((page) => !t(page.labelKey).toLowerCase().includes(normalizedFilter)) ? (
              <p className="px-1 text-sm text-muted-foreground" role="status">{t("settings.compactNav.noResults")}</p>
            ) : null}
          </div>
        </Sheet>
      ) : null}
    </div>
  );
}
