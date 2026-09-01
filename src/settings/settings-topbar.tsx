import { ArrowLeft } from "lucide-react";
import { useTranslation } from "react-i18next";
import { Button } from "../components/ui/button";
import { settingsPages, type SettingsPageDefinition } from "./settings-pages";
import { SettingsSearchBox } from "./settings-search-box";
import type { SettingsSearchEntry, SettingsSearchResult } from "./settings-search-index";

interface SettingsTopBarProps {
  activePage: SettingsPageDefinition;
  searchTerm: string;
  searchIndex: SettingsSearchEntry[];
  onSearchTermChange: (value: string) => void;
  onSelectSearchResult: (result: SettingsSearchResult) => void;
  onReturn?: () => void;
}

export function SettingsTopBar({ activePage, searchTerm, searchIndex, onSearchTermChange, onSelectSearchResult, onReturn }: SettingsTopBarProps) {
  const { t } = useTranslation();

  return (
    <header className="flex min-h-16 flex-col gap-3 border-b border-border bg-background px-3 py-3 sm:px-4 lg:flex-row lg:items-center lg:justify-between">
      <div className="flex min-w-0 items-center gap-3">
        <div className="flex h-9 w-9 shrink-0 items-center justify-center rounded-md border border-primary/30 bg-[hsl(var(--nav-active-soft))] text-sm font-bold text-primary">
          V
        </div>
        <div className="min-w-0">
          {/* Task 12.8: this used to also render `<h1>{t(activePage.crumbKey)}</h1>` here, the
              same page title every page's own header (`page-parts.tsx`'s `PageHeader`, an `<h2>`)
              already presents -- two headings naming the same page. The active page's own header
              is the one authoritative title now; this stays app-level chrome only. */}
          <div className="flex min-w-0 flex-wrap items-center gap-x-2 gap-y-1 text-xs text-muted-foreground">
            <span className="font-medium text-foreground">VaneHub AI</span>
            <span>/</span>
            <span>{t("app.settings.breadcrumb")}</span>
          </div>
        </div>
      </div>

      <div className="grid min-w-0 gap-2 sm:grid-cols-[minmax(220px,360px)_auto] lg:flex lg:flex-1 lg:items-center lg:justify-end">
        <div className="min-w-0 lg:w-[min(34vw,420px)] lg:min-w-72">
          <SettingsSearchBox
            index={searchIndex}
            onSearchTermChange={onSearchTermChange}
            onSelectResult={onSelectSearchResult}
            pages={settingsPages}
            placeholder={t(activePage.searchPlaceholderKey)}
            searchTerm={searchTerm}
          />
        </div>

        <Button className="justify-center" variant="outline" onClick={onReturn} size="sm">
          <ArrowLeft className="h-4 w-4" aria-hidden="true" />
          {t("app.settings.return")}
        </Button>
      </div>
    </header>
  );
}
