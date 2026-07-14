import { ArrowLeft, Search } from "lucide-react";
import { Button } from "../components/ui/button";
import type { SettingsPageDefinition } from "./settings-pages";

interface SettingsTopBarProps {
  activePage: SettingsPageDefinition;
  searchTerm: string;
  onSearchTermChange: (value: string) => void;
  onReturn?: () => void;
}

export function SettingsTopBar({ activePage, searchTerm, onSearchTermChange, onReturn }: SettingsTopBarProps) {
  return (
    <header className="ucd-panel mx-2 mt-2 flex min-h-12 flex-wrap items-center justify-between gap-3 rounded-md px-3 py-2">
      <div className="flex min-w-0 items-center gap-2">
        <div className="flex h-7 w-7 shrink-0 items-center justify-center rounded-md border border-primary text-sm font-bold text-primary">
          V
        </div>
        <div className="flex min-w-0 flex-wrap items-center gap-2 text-sm">
          <span className="font-semibold text-foreground">VaneHub AI</span>
          <span className="text-muted-foreground">/</span>
          <span className="text-muted-foreground">设置</span>
          <span className="text-muted-foreground">/</span>
          <span className="font-semibold">{activePage.crumb}</span>
        </div>
      </div>

      <div className="flex flex-1 flex-wrap items-center justify-end gap-2">
        <div className="relative min-w-56 max-w-xs flex-1 sm:flex-none">
          <Search className="pointer-events-none absolute left-3 top-1/2 h-4 w-4 -translate-y-1/2 text-muted-foreground" />
          <input
            className="ucd-input h-8 w-full rounded px-9 text-sm outline-none focus-visible:ring-2 focus-visible:ring-ring"
            onChange={(event) => onSearchTermChange(event.target.value)}
            placeholder={activePage.searchPlaceholder}
            value={searchTerm}
          />
        </div>

        <Button variant="outline" onClick={onReturn}>
          <ArrowLeft className="h-4 w-4" aria-hidden="true" />
          返回
        </Button>
      </div>
    </header>
  );
}
