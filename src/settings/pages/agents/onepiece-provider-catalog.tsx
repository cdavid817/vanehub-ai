import { useTranslation } from "react-i18next";
import { ProviderCatalog } from "../../../components/provider-directory/provider-catalog";
import { ProviderEndpointBadge } from "../../../components/provider-directory/provider-endpoint-controls";
import type { OnePieceProviderPreset } from "../../../types/agent";

export function OnePieceProviderCatalog({ presets, selectedPresetId, onSelect }: {
  presets: OnePieceProviderPreset[];
  selectedPresetId: string | null;
  onSelect: (preset: OnePieceProviderPreset) => void;
}) {
  const { t } = useTranslation();
  return <ProviderCatalog
    description={t("onepiece.catalog.description")}
    emptyLabel={t("onepiece.catalog.empty")}
    items={presets.map((preset) => ({
      id: preset.id,
      displayName: preset.displayName,
      category: preset.category,
      iconKey: preset.iconKey,
      catalogVersion: preset.catalogVersion,
      searchText: `${preset.provider} ${preset.defaultModelId} ${preset.endpoints.map((endpoint) => endpoint.type).join(" ")}`,
      detail: <span className="flex flex-wrap items-center gap-1.5"><span className="mr-1 truncate" title={preset.defaultModelId}>{preset.defaultModelId}</span>{preset.endpoints.filter((endpoint) => endpoint.type !== "openai-responses").map((endpoint) => <ProviderEndpointBadge key={endpoint.type} type={endpoint.type} />)}</span>,
    }))}
    onSelect={(id) => { const preset = presets.find((candidate) => candidate.id === id); if (preset) onSelect(preset); }}
    searchLabel={t("onepiece.catalog.search")}
    selectedId={selectedPresetId}
    title={t("onepiece.catalog.title")}
  />;
}
