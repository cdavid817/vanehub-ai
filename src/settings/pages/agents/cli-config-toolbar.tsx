import { Download, Plus, RefreshCw, Search } from "lucide-react";
import { useTranslation } from "react-i18next";
import { Button } from "../../../components/ui/button";

export function CliConfigToolbar({
  busy,
  onAdd,
  onImport,
  onRefresh,
  onSearchChange,
  search,
}: {
  busy: boolean;
  onAdd: () => void;
  onImport: () => void;
  onRefresh: () => void;
  onSearchChange: (value: string) => void;
  search: string;
}) {
  const { t } = useTranslation();

  return (
    <div className="flex flex-col gap-3 sm:flex-row sm:items-center">
      <div className="relative min-w-0 flex-1">
        <Search className="absolute left-3 top-1/2 h-4 w-4 -translate-y-1/2 text-muted-foreground" />
        <input
          aria-label={t("agentConfigurations.toolbar.search")}
          className="ucd-input h-10 w-full rounded-lg pl-9 pr-3 text-sm"
          onChange={(event) => onSearchChange(event.target.value)}
          placeholder={t("agentConfigurations.toolbar.search")}
          value={search}
        />
      </div>
      <div className="grid grid-cols-3 gap-2 sm:flex">
        <Button data-testid="cli-config-add-profile" disabled={busy} onClick={onAdd}><Plus className="h-4 w-4" />{t("agentConfigurations.toolbar.add")}</Button>
        <Button disabled={busy} onClick={onImport} variant="outline"><Download className="h-4 w-4" />{t("agents.globalConfig.import")}</Button>
        <Button aria-label={t("agentConfigurations.refresh")} disabled={busy} onClick={onRefresh} variant="outline"><RefreshCw className="h-4 w-4" /><span className="sm:hidden lg:inline">{t("agentConfigurations.refresh")}</span></Button>
      </div>
    </div>
  );
}
