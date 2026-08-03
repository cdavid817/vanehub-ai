import { Plus } from "lucide-react";
import { useMemo } from "react";
import { useTranslation } from "react-i18next";
import { ProviderCatalog } from "../../../components/provider-directory/provider-catalog";
import { ProviderEndpointBadge, ProviderEndpointSelector, ProviderHelpLinks } from "../../../components/provider-directory/provider-endpoint-controls";
import { Button } from "../../../components/ui/button";
import { getOnePieceProviderPresets } from "../../../config/onepiece-provider-presets";
import type { CliConfigPreset } from "../../../types/cli-agent-config";

function presetBaseUrl(preset: CliConfigPreset) { return preset.payload.baseUrl; }

export function CliConfigProviderCatalog({ presets, selectedPresetId, onCreateCustom, onSelectPreset, onOpenUrl = () => undefined }: {
  presets: CliConfigPreset[];
  selectedPresetId: string | null;
  onCreateCustom: () => void;
  onSelectPreset: (preset: CliConfigPreset) => void;
  onOpenUrl?: (url: string) => void;
}) {
  const { t } = useTranslation();
  const groups = useMemo(() => {
    const map = new Map<string, CliConfigPreset[]>();
    for (const preset of presets) map.set(preset.providerId, [...(map.get(preset.providerId) ?? []), preset]);
    return [...map.entries()].map(([providerId, entries]) => {
      const sorted = [...entries].sort((left, right) => {
        if (left.agentId !== "codex-cli") return 0;
        const rank = { "openai-responses": 0, "openai-chat-completions": 1, "anthropic-messages": 2 } as const;
        return rank[left.endpointType] - rank[right.endpointType];
      });
      return { providerId, entries: sorted, primary: sorted[0] };
    });
  }, [presets]);
  const selectedPreset = presets.find((preset) => preset.id === selectedPresetId) ?? null;
  const selectedProviderId = selectedPreset?.providerId ?? null;
  const selectedGroup = groups.find((group) => group.providerId === selectedProviderId) ?? null;
  const providerMetadata = getOnePieceProviderPresets().find((provider) => provider.id === selectedProviderId) ?? null;

  return <div className="grid gap-3">
    <ProviderCatalog
      description={t("agentConfigurations.providers.description")}
      emptyLabel={t("agentConfigurations.providers.empty")}
      items={groups.map(({ providerId, entries, primary }) => ({
        id: providerId,
        displayName: primary.displayName,
        category: primary.category === "official" ? "official" : "common",
        iconKey: primary.iconKey,
        catalogVersion: primary.catalogVersion,
        searchText: `${primary.description} ${entries.map((entry) => entry.endpointType).join(" ")}`,
        detail: <><span className="block">{primary.description}</span><span className="mt-1.5 flex flex-wrap gap-1.5">{entries.map((entry) => <ProviderEndpointBadge key={entry.endpointType} type={entry.endpointType} />)}</span></>,
      }))}
      onSelect={(providerId) => { const group = groups.find((candidate) => candidate.providerId === providerId); if (group) onSelectPreset(group.entries[0]); }}
      searchLabel={t("agents.globalConfig.searchPresets")}
      selectedId={selectedProviderId}
      title={t("agentConfigurations.dialog.create.chooseProvider")}
    />
    {selectedGroup && selectedPreset ? <ProviderEndpointSelector endpoints={selectedGroup.entries.map((preset) => ({ type: preset.endpointType, baseUrl: presetBaseUrl(preset) }))} label={t("onepiece.endpoint")} onChange={(type) => { const preset = selectedGroup.entries.find((candidate) => candidate.endpointType === type); if (preset) onSelectPreset(preset); }} value={selectedPreset.endpointType} /> : null}
    {providerMetadata ? <ProviderHelpLinks apiKeyLabel={t("onepiece.openApiKeyPage")} apiKeyUrl={providerMetadata.apiKeyUrl} docsLabel={t("onepiece.openProviderDocs")} docsUrl={providerMetadata.docsUrl} onOpenUrl={onOpenUrl} /> : null}
    <Button aria-pressed={selectedPresetId === null} className="w-full" onClick={onCreateCustom} variant={selectedPresetId === null ? "default" : "outline"}><Plus className="h-4 w-4" />{t("agents.globalConfig.custom")}</Button>
  </div>;
}
