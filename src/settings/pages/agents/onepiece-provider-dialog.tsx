import { useMemo, useRef, useState } from "react";
import { useMutation } from "@tanstack/react-query";
import { LoaderCircle, RefreshCw, Search } from "lucide-react";
import { useTranslation } from "react-i18next";
import { ApplicationDialog } from "../../../components/ui/application-dialog";
import { Button } from "../../../components/ui/button";
import { ProviderBrandIcon } from "../../../components/provider-brand-icon";
import { ProviderConfigurationSection, ProviderEndpointSelector, ProviderHelpLinks } from "../../../components/provider-directory/provider-endpoint-controls";
import { ProviderCredentialValidation } from "../../../components/provider-directory/provider-credential-validation";
import type { AgentService } from "../../../services/agent-service";
import { agentService as defaultAgentService } from "../../../services/runtime-agent-client";
import type { OnePieceProviderModelOption, OnePieceProviderPreset, OnePieceProviderProfile, OnePieceProviderProfiles, ProviderEndpointType } from "../../../types/agent";
import { OnePieceProviderCatalog } from "./onepiece-provider-catalog";

const inputClass = "ucd-input h-9 rounded px-3 text-sm outline-hidden focus-visible:ring-2 focus-visible:ring-ring";

function initialModelOptions(preset: OnePieceProviderPreset | null, profile: OnePieceProviderProfile | null): OnePieceProviderModelOption[] {
  const ids = [profile?.modelId, ...(preset?.fallbackModels ?? [])].filter((value): value is string => Boolean(value));
  return [...new Set(ids)].map((id) => ({ id, displayName: id, source: id === profile?.modelId ? "profile" : "catalog" }));
}

export function OnePieceProviderDialog({ profile, presets, onClose, onSaved, service = defaultAgentService }: {
  profile: OnePieceProviderProfile | null;
  presets: OnePieceProviderPreset[];
  onClose: () => void;
  onSaved: (overview: OnePieceProviderProfiles) => void;
  service?: AgentService;
}) {
  const { t } = useTranslation();
  const profileProviderId = profile?.sourceProviderId ?? null;
  const profilePreset = presets.find((preset) => preset.id === profileProviderId) ?? null;
  const [selectedPreset, setSelectedPreset] = useState<OnePieceProviderPreset | null>(profilePreset);
  const [endpointType, setEndpointType] = useState<ProviderEndpointType>(() => {
    return profile?.sourceEndpointType ?? profilePreset?.defaultEndpointType ?? "openai-chat-completions";
  });
  const [name, setName] = useState(profile?.name ?? profilePreset?.displayName ?? "");
  const [modelId, setModelId] = useState(profile?.modelId ?? profilePreset?.defaultModelId ?? "");
  const [modelOptions, setModelOptions] = useState(() => initialModelOptions(profilePreset, profile));
  const [modelSearch, setModelSearch] = useState("");
  const [modelWarning, setModelWarning] = useState<string | null>(null);
  const [apiKey, setApiKey] = useState("");
  const [error, setError] = useState<string | null>(null);
  const [validationRevision, setValidationRevision] = useState(0);
  const latestDiscovery = useRef(0);
  const saveMutation = useMutation({
    mutationFn: () => service.saveOnePieceProviderProfile({
      id: profile?.id ?? null,
      name: name.trim(),
      providerId: selectedPreset?.id ?? "",
      endpointType,
      modelId: modelId.trim(),
      apiKey: apiKey.trim() || null,
    }),
    onSuccess: onSaved,
    onError: (reason) => setError(reason instanceof Error ? reason.message : String(reason)),
  });
  const discoverMutation = useMutation({
    mutationFn: async () => {
      const requestId = ++latestDiscovery.current;
      const result = await service.discoverOnePieceProviderModels({
        providerId: selectedPreset?.id ?? "",
        endpointType,
        profileId: profile?.id ?? null,
        apiKey: apiKey.trim() || null,
      });
      return { requestId, result };
    },
    onSuccess: ({ requestId, result }) => {
      if (requestId !== latestDiscovery.current) return;
      if (result.providerId !== selectedPreset?.id || result.endpointType !== endpointType) return;
      setModelOptions(result.models);
      setModelWarning(result.warning);
      setError(null);
    },
    onError: (reason) => setError(reason instanceof Error ? reason.message : String(reason)),
  });
  const filteredModelOptions = useMemo(() => {
    const query = modelSearch.trim().toLocaleLowerCase();
    return modelOptions.filter((option) => option.id === modelId
      || !query
      || `${option.displayName} ${option.id}`.toLocaleLowerCase().includes(query));
  }, [modelId, modelOptions, modelSearch]);
  const missingRequiredValue = !selectedPreset || !name.trim() || !modelId.trim()
    || (!profile?.credentialPresent && !apiKey.trim());

  function selectPreset(preset: OnePieceProviderPreset) {
    latestDiscovery.current += 1;
    setSelectedPreset(preset);
    setEndpointType(preset.defaultEndpointType);
    setName(preset.displayName);
    setModelId(preset.defaultModelId);
    setModelOptions(initialModelOptions(preset, null));
    setModelSearch("");
    setModelWarning(null);
    setError(null);
    setValidationRevision((value) => value + 1);
  }

  return (
    <ApplicationDialog closeDisabled={saveMutation.isPending} description={t("onepiece.dialog.description")} maxWidth="max-w-4xl" onClose={onClose} title={profile ? t("onepiece.dialog.editTitle") : t("onepiece.dialog.addTitle")}>
      <div className="grid gap-4">
        {!profile || !profilePreset ? <OnePieceProviderCatalog onSelect={selectPreset} presets={presets} selectedPresetId={selectedPreset?.id ?? null} /> : (
          <div className="flex items-center gap-3 rounded-xl border border-border bg-muted/25 p-3 sm:p-4"><ProviderBrandIcon iconKey={profilePreset.iconKey} label={profilePreset.displayName} /><div className="min-w-0"><p className="text-xs text-muted-foreground">{t("onepiece.catalog.selected")}</p><p className="truncate font-semibold">{profilePreset.displayName}</p></div></div>
        )}
        {selectedPreset ? <ProviderEndpointSelector disabled={Boolean(profile)} endpoints={selectedPreset.endpoints.filter((endpoint) => endpoint.type !== "openai-responses")} label={t("onepiece.endpoint")} onChange={(type) => { setEndpointType(type); setModelWarning(null); setValidationRevision((value) => value + 1); }} value={endpointType} /> : null}
        <ProviderConfigurationSection description={t("agentConfigurations.providers.detailsDescription")} title={t("agentConfigurations.providers.detailsTitle")}><div className="grid gap-4 sm:grid-cols-2">
          <label className="flex flex-col gap-1 text-sm">{t("onepiece.profileName")}<input data-dialog-autofocus className={inputClass} value={name} onChange={(event) => setName(event.target.value)} /></label>
          <div className="flex flex-col gap-2 text-sm">
            <span>{t("onepiece.model")}</span>
            <div className="relative"><Search className="absolute left-3 top-2.5 h-4 w-4 text-muted-foreground" /><input aria-label={t("onepiece.modelSearch")} className={`${inputClass} w-full pl-9`} onChange={(event) => setModelSearch(event.target.value)} placeholder={t("onepiece.modelSearch")} value={modelSearch} /></div>
            <select aria-label={t("onepiece.model")} className={inputClass} onChange={(event) => { setModelId(event.target.value); setValidationRevision((value) => value + 1); }} value={modelId}>
              {filteredModelOptions.map((option) => <option key={option.id} value={option.id}>{option.displayName === option.id ? option.id : `${option.displayName} (${option.id})`}</option>)}
            </select>
            <Button disabled={discoverMutation.isPending || !selectedPreset || (!apiKey.trim() && !profile?.credentialPresent)} onClick={() => void discoverMutation.mutateAsync().catch(() => undefined)} type="button" variant="outline">{discoverMutation.isPending ? <LoaderCircle className="h-4 w-4 animate-spin" /> : <RefreshCw className="h-4 w-4" />}{discoverMutation.isPending ? t("onepiece.fetchingModels") : t("onepiece.fetchModels")}</Button>
          </div>
          <label className="flex flex-col gap-1 text-sm sm:col-span-2">{t("onepiece.apiKey")}<input autoComplete="new-password" className={inputClass} onChange={(event) => { latestDiscovery.current += 1; setApiKey(event.target.value); setModelWarning(null); setValidationRevision((value) => value + 1); }} placeholder={profile?.credentialPresent ? t("onepiece.keyOptional") : t("onepiece.keyRequired")} type="password" value={apiKey} /></label>
          <div className="sm:col-span-2">
            <ProviderCredentialValidation
              disabled={!selectedPreset || !modelId.trim() || (!apiKey.trim() && !profile?.credentialPresent)}
              onValidate={() => service.validateOnePieceProviderCredential({
                providerId: selectedPreset?.id ?? "",
                endpointType,
                modelId: modelId.trim(),
                profileId: profile?.id ?? null,
                apiKey: apiKey.trim() || null,
              })}
              resetKey={validationRevision}
            />
          </div>
        </div></ProviderConfigurationSection>
        {selectedPreset ? <ProviderHelpLinks apiKeyLabel={t("onepiece.openApiKeyPage")} apiKeyUrl={selectedPreset.apiKeyUrl} docsLabel={t("onepiece.openProviderDocs")} docsUrl={selectedPreset.docsUrl} onOpenUrl={(url) => { void service.openExternalUrl(url).catch((reason) => setError(reason instanceof Error ? reason.message : String(reason))); }} /> : null}
      </div>
      {modelWarning ? <p className="mt-3 text-sm ucd-status-warning" role="status">{t("onepiece.modelDiscoveryFallback")}</p> : null}
      {error ? <p className="mt-3 text-sm ucd-status-warning" role="alert">{error}</p> : null}
      <div className="sticky -bottom-5 -mx-5 mt-5 flex justify-end gap-2 border-t border-border bg-background px-5 pb-5 pt-4 sm:-bottom-6 sm:-mx-6 sm:px-6 sm:pb-6">
        <Button disabled={saveMutation.isPending} onClick={onClose} variant="outline">{t("agents.edit.cancel")}</Button>
        <Button disabled={saveMutation.isPending || missingRequiredValue} onClick={() => void saveMutation.mutateAsync().catch(() => undefined)}>{saveMutation.isPending ? t("agents.edit.saving") : t("onepiece.save")}</Button>
      </div>
    </ApplicationDialog>
  );
}
