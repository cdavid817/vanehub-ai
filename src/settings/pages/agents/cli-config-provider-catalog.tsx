import { Check, Plus, Search } from "lucide-react";
import { useMemo, useState } from "react";
import { useTranslation } from "react-i18next";
import { Badge } from "../../../components/ui/badge";
import { Button } from "../../../components/ui/button";
import type { CliConfigPreset, CliConfigPresetCategory } from "../../../types/cli-agent-config";

type CatalogFilter = "all" | Extract<CliConfigPresetCategory, "official" | "common">;

export function CliConfigProviderCatalog({
  presets,
  selectedPresetId,
  onCreateCustom,
  onSelectPreset,
}: {
  presets: CliConfigPreset[];
  selectedPresetId: string | null;
  onCreateCustom: () => void;
  onSelectPreset: (preset: CliConfigPreset) => void;
}) {
  const { t } = useTranslation();
  const [search, setSearch] = useState("");
  const [category, setCategory] = useState<CatalogFilter>("all");
  const filtered = useMemo(() => {
    const query = search.trim().toLowerCase();
    return presets.filter((preset) =>
      (category === "all" || preset.category === category)
      && (!query || `${preset.displayName} ${preset.description}`.toLowerCase().includes(query)),
    );
  }, [category, presets, search]);

  return (
    <section aria-labelledby="provider-preset-heading" className="rounded-lg border border-border bg-muted/25 p-4">
      <div>
        <h3 className="font-semibold" id="provider-preset-heading">{t("agentConfigurations.dialog.create.chooseProvider")}</h3>
        <p className="mt-1 text-sm leading-6 text-muted-foreground">{t("agentConfigurations.providers.description")}</p>
      </div>
      <div className="relative mt-4">
        <Search className="absolute left-3 top-3 h-4 w-4 text-muted-foreground" />
        <input aria-label={t("agents.globalConfig.searchPresets")} className="ucd-input h-10 w-full rounded-md pl-9 pr-3 text-sm" onChange={(event) => setSearch(event.target.value)} placeholder={t("agents.globalConfig.searchPresets")} value={search} />
      </div>
      <div className="mt-3 flex flex-wrap gap-2" role="group" aria-label={t("agentConfigurations.providers.categories")}>
        {(["all", "official", "common"] as const).map((candidate) => <Button className="h-8 px-3 text-xs" key={candidate} onClick={() => setCategory(candidate)} variant={category === candidate ? "default" : "outline"}>{t(`agentConfigurations.providers.category.${candidate}`)}</Button>)}
      </div>
      <div className="mt-4 grid grid-cols-[repeat(auto-fill,minmax(150px,1fr))] gap-2">
        {filtered.map((preset) => (
          <button aria-pressed={selectedPresetId === preset.id} className={`group relative min-h-24 rounded-md border p-3 text-left transition-colors ${selectedPresetId === preset.id ? "border-primary bg-primary/5 ring-1 ring-primary/30" : "border-border bg-background hover:border-primary/50 hover:bg-muted/50"}`} key={preset.id} onClick={() => onSelectPreset(preset)} type="button">
            {selectedPresetId === preset.id ? <Check className="absolute right-2 top-2 h-4 w-4 text-primary" /> : null}
            <span className="block pr-5 font-medium group-hover:text-primary">{preset.displayName}</span>
            <span className="mt-2 flex flex-wrap gap-1.5"><Badge tone={preset.category === "official" ? "success" : "muted"}>{t(`agents.globalConfig.presetCategory.${preset.category}`)}</Badge><Badge tone="muted">v{preset.catalogVersion}</Badge></span>
            <span className="mt-1 block text-xs leading-5 text-muted-foreground">{preset.description}</span>
            {preset.deprecated ? <span className="mt-2 block text-xs ucd-status-warning">{t("agents.globalConfig.deprecated")}</span> : null}
          </button>
        ))}
        {filtered.length === 0 ? <p className="rounded-md border border-dashed border-border p-4 text-sm text-muted-foreground">{t("agentConfigurations.providers.empty")}</p> : null}
      </div>
      <Button aria-pressed={selectedPresetId === null} className="mt-3 w-full" onClick={onCreateCustom} variant={selectedPresetId === null ? "default" : "outline"}><Plus className="h-4 w-4" />{t("agents.globalConfig.custom")}</Button>
    </section>
  );
}
