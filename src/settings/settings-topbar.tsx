import { ArrowLeft, Search } from "lucide-react";
import { useTranslation } from "react-i18next";
import { Button } from "../components/ui/button";
import type { SettingsPageDefinition } from "./settings-pages";

interface SettingsTopBarProps {
  activePage: SettingsPageDefinition;
  searchTerm: string;
  onSearchTermChange: (value: string) => void;
  onReturn?: () => void;
}

export function SettingsTopBar({ activePage, searchTerm, onSearchTermChange, onReturn }: SettingsTopBarProps) {
  const { t } = useTranslation();

  return (
    <header className="ucd-panel mx-2 mt-2 flex min-h-12 flex-wrap items-center justify-between gap-3 rounded-lg px-3 py-2">
      <div className="flex min-w-0 items-center gap-2">
        <div className="flex h-8 w-8 shrink-0 items-center justify-center rounded-md border border-primary bg-[hsl(var(--nav-active-soft))] text-sm font-bold text-primary">
          V
        </div>
        <div className="flex min-w-0 flex-wrap items-center gap-2 text-sm">
          <span className="font-semibold text-foreground">VaneHub AI</span>
          <span className="text-muted-foreground">/</span>
          <span className="text-muted-foreground">{t("app.settings.breadcrumb")}</span>
          <span className="text-muted-foreground">/</span>
          <span className="font-semibold">{t(activePage.crumbKey)}</span>
        </div>
      </div>

      <div className="flex flex-1 flex-wrap items-center justify-end gap-2">
        <div className="relative min-w-56 max-w-xs flex-1 sm:flex-none">
          <Search className="pointer-events-none absolute left-3 top-1/2 h-4 w-4 -translate-y-1/2 text-muted-foreground" />
          <input
            className="ucd-input h-8 w-full rounded-md px-9 text-sm outline-none focus-visible:ring-2 focus-visible:ring-ring"
            onChange={(event) => onSearchTermChange(event.target.value)}
            placeholder={t(activePage.searchPlaceholderKey)}
            value={searchTerm}
          />
        </div>

        <Button variant="outline" onClick={onReturn} size="sm">
          <ArrowLeft className="h-4 w-4" aria-hidden="true" />
          {t("app.settings.return")}
        </Button>
      </div>
    </header>
  );
}
