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
    <header className="flex min-h-16 flex-col gap-3 border-b border-border bg-background px-3 py-3 sm:px-4 lg:flex-row lg:items-center lg:justify-between">
      <div className="flex min-w-0 items-center gap-3">
        <div className="flex h-9 w-9 shrink-0 items-center justify-center rounded-md border border-primary/30 bg-[hsl(var(--nav-active-soft))] text-sm font-bold text-primary">
          V
        </div>
        <div className="min-w-0">
          <div className="flex min-w-0 flex-wrap items-center gap-x-2 gap-y-1 text-xs text-muted-foreground">
            <span className="font-medium text-foreground">VaneHub AI</span>
            <span>/</span>
            <span>{t("app.settings.breadcrumb")}</span>
          </div>
          <h1 className="mt-0.5 break-words text-lg font-semibold leading-tight tracking-tight">{t(activePage.crumbKey)}</h1>
        </div>
      </div>

      <div className="grid min-w-0 gap-2 sm:grid-cols-[minmax(220px,360px)_auto] lg:flex lg:flex-1 lg:items-center lg:justify-end">
        <div className="relative min-w-0 lg:w-[min(34vw,420px)] lg:min-w-72">
          <Search className="pointer-events-none absolute left-3 top-1/2 h-4 w-4 -translate-y-1/2 text-muted-foreground" />
          <input
            className="ucd-input h-9 w-full rounded-md px-9 text-sm outline-none focus-visible:ring-2 focus-visible:ring-ring"
            onChange={(event) => onSearchTermChange(event.target.value)}
            placeholder={t(activePage.searchPlaceholderKey)}
            value={searchTerm}
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
