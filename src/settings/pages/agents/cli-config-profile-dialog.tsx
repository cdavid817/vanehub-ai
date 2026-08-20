import { useState } from "react";
import { useMutation } from "@tanstack/react-query";
import { useTranslation } from "react-i18next";
import { ChevronLeft, ChevronRight } from "lucide-react";
import { Button } from "../../../components/ui/button";
import { ApplicationDialog } from "../../../components/ui/application-dialog";
import { createCustomCliConfigPayload } from "../../../config/cli-agent-provider-presets";
import type { AgentService } from "../../../services/agent-service";
import { agentService as defaultAgentService } from "../../../services/runtime-agent-client";
import type {
  CliConfigAgentId,
  CliConfigPayload,
  CliConfigPreset,
  CliConfigProfile,
} from "../../../types/cli-agent-config";
import { CliConfigPayloadFields, payloadHasAdvancedFields } from "./cli-config-payload-fields";
import { CliConfigProviderCatalog } from "./cli-config-provider-catalog";
import { ProviderConfigurationSection } from "../../../components/provider-directory/provider-endpoint-controls";
import { ProviderCredentialValidation } from "../../../components/provider-directory/provider-credential-validation";
import { ProviderBrandIcon } from "../../../components/provider-brand-icon";

export interface CliConfigProfileDraft {
  agentId: CliConfigAgentId;
  profile: CliConfigProfile | null;
  preset: CliConfigPreset | null;
  payload: CliConfigPayload;
}

export function CliConfigProfileDialog({
  draft,
  presets,
  onClose,
  onSaved,
  service = defaultAgentService,
}: {
  draft: CliConfigProfileDraft;
  presets: CliConfigPreset[];
  onClose: () => void;
  onSaved: (profile: CliConfigProfile) => void;
  service?: AgentService;
}) {
  const { t } = useTranslation();
  const [name, setName] = useState(draft.profile?.name ?? draft.preset?.displayName ?? "");
  const [payload, setPayload] = useState<CliConfigPayload>(() => structuredClone(draft.payload));
  const [selectedPreset, setSelectedPreset] = useState<CliConfigPreset | null>(draft.preset);
  const [customSelected, setCustomSelected] = useState(Boolean(draft.profile && !draft.profile.sourcePresetId));
  const [credential, setCredential] = useState("");
  const [removeCredential, setRemoveCredential] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [validationRevision, setValidationRevision] = useState(0);
  const [stage, setStage] = useState<"provider" | "configuration">(draft.profile || draft.preset ? "configuration" : "provider");

  const saveMutation = useMutation({
    mutationFn: () => service.saveCliConfigProfile({
      id: draft.profile?.id ?? null,
      agentId: draft.agentId,
      name: name.trim(),
      payload,
      sourcePresetId: selectedPreset?.id ?? draft.profile?.sourcePresetId ?? null,
      sourcePresetVersion: selectedPreset?.catalogVersion ?? draft.profile?.sourcePresetVersion ?? null,
      credential: credential.trim() || null,
      removeCredential,
    }),
    onSuccess: onSaved,
    onError: (reason) => setError(reason instanceof Error ? reason.message : String(reason)),
  });

  function selectPreset(preset: CliConfigPreset) {
    setCustomSelected(false);
    setSelectedPreset(preset);
    setName(preset.displayName);
    setPayload(structuredClone(preset.payload));
    setError(null);
    setValidationRevision((value) => value + 1);
  }

  function selectCustom() {
    setCustomSelected(true);
    setSelectedPreset(null);
    setName("");
    setPayload(createCustomCliConfigPayload(draft.agentId));
    setError(null);
    setValidationRevision((value) => value + 1);
  }

  const usesCredential = payload.kind === "claude-code"
    ? payload.authMode !== "none"
    : payload.kind === "codex-cli"
      ? payload.authStrategy !== "preserve-official"
      : payload.kind === "gemini-cli"
        ? payload.authStrategy !== "preserve-official"
        : payload.kind !== "antigravity";
  const validationDisabled = removeCredential
    || (usesCredential && !credential.trim() && !draft.profile?.credentialConfigured);
  const providerLabel = selectedPreset?.displayName
    ?? (customSelected ? t("agents.globalConfig.custom") : draft.profile?.name ?? "");
  const providerIcon = selectedPreset?.iconKey ?? (customSelected ? "custom-endpoint" : "unknown");

  return (
    <ApplicationDialog closeDisabled={saveMutation.isPending} description={t("agents.globalConfig.draftHint")} maxWidth="max-w-4xl" onClose={onClose} title={draft.profile ? t("agents.globalConfig.editProfile") : t("agents.globalConfig.createProfile")}>
        {!draft.profile ? <ol aria-label={t("agentConfigurations.dialog.progress")} className="mb-4 grid grid-cols-2 gap-2 text-xs">
          {(["provider", "configuration"] as const).map((candidate, index) => <li className={`rounded-md border px-3 py-2 ${stage === candidate ? "border-primary/50 bg-primary/5 font-semibold text-foreground" : "border-border text-muted-foreground"}`} key={candidate}>{index + 1}. {t(`agentConfigurations.dialog.step.${candidate}`)}</li>)}
        </ol> : null}

        {stage === "provider" ? (
          <div data-testid="cli-config-provider-stage"><CliConfigProviderCatalog customSelected={customSelected} onCreateCustom={selectCustom} onOpenUrl={(url) => { void service.openExternalUrl(url).catch((reason) => setError(reason instanceof Error ? reason.message : String(reason))); }} onSelectPreset={selectPreset} presets={presets} selectedPresetId={selectedPreset?.id ?? null} /></div>
        ) : (
        <div className="grid gap-3" data-testid="cli-config-configuration-stage">
          <div className="flex items-center gap-3 rounded-lg border border-border bg-muted/20 px-3 py-2.5">
            <ProviderBrandIcon iconKey={providerIcon} label={providerLabel} />
            <div className="min-w-0"><p className="text-xs text-muted-foreground">{t("agentConfigurations.dialog.selectedProvider")}</p><p className="truncate text-sm font-semibold">{providerLabel}</p></div>
          </div>
          <ProviderConfigurationSection description={t("agentConfigurations.providers.detailsDescription")} title={t("agentConfigurations.providers.detailsTitle")}><label className="flex flex-col gap-1 text-sm">
            {t("agents.globalConfig.profileName")}
            <input data-dialog-autofocus className="ucd-input h-9 rounded px-3 text-sm outline-hidden focus-visible:ring-2 focus-visible:ring-ring" onChange={(event) => setName(event.target.value)} value={name} />
          </label>
          <CliConfigPayloadFields payload={payload} onChange={(value) => { setPayload(value); setValidationRevision((revision) => revision + 1); }} />
          {payloadHasAdvancedFields(payload) ? <details className="rounded-lg border border-border bg-muted/15 p-3" data-testid="cli-config-advanced-settings">
            <summary className="cursor-pointer text-sm font-medium">{t("agentConfigurations.dialog.advanced")}</summary>
            <p className="mb-3 mt-1 text-xs text-muted-foreground">{t("agentConfigurations.dialog.advancedDescription")}</p>
            <CliConfigPayloadFields payload={payload} section="advanced" onChange={(value) => { setPayload(value); setValidationRevision((revision) => revision + 1); }} />
          </details> : null}
          {payload.kind !== "antigravity" ? <><label className="flex flex-col gap-1 text-sm">
            {t("agents.globalConfig.credential")}
            <input autoComplete="new-password" className="ucd-input h-9 rounded px-3 text-sm outline-hidden focus-visible:ring-2 focus-visible:ring-ring" onChange={(event) => { setCredential(event.target.value); setValidationRevision((value) => value + 1); }} placeholder={draft.profile?.credentialConfigured ? t("agents.globalConfig.credentialKeep") : t("agents.globalConfig.credentialPlaceholder")} type="password" value={credential} />
          </label>
          <ProviderCredentialValidation
            disabled={validationDisabled}
            onValidate={() => service.validateCliConfigCredential({
              agentId: draft.agentId,
              profileId: draft.profile?.id ?? null,
              payload,
              sourcePresetId: selectedPreset?.id ?? draft.profile?.sourcePresetId ?? null,
              credential: credential.trim() || null,
            })}
            resetKey={validationRevision}
          />
          {draft.profile?.credentialConfigured ? (
            <label className="flex items-center gap-2 text-sm">
              <input checked={removeCredential} onChange={(event) => { setRemoveCredential(event.target.checked); setValidationRevision((value) => value + 1); }} type="checkbox" />
              {t("agents.globalConfig.removeCredential")}
            </label>
          ) : null}</> : null}</ProviderConfigurationSection>
        </div>
        )}

        {error ? <p className="mt-3 text-sm ucd-status-warning" role="alert">{error}</p> : null}
        <div className="sticky -bottom-5 -mx-5 mt-5 flex justify-end gap-2 border-t border-border bg-background px-5 pb-5 pt-4 sm:-bottom-6 sm:-mx-6 sm:px-6 sm:pb-6">
          {stage === "configuration" && !draft.profile ? <Button disabled={saveMutation.isPending} onClick={() => setStage("provider")} variant="ghost"><ChevronLeft className="h-4 w-4" />{t("agentConfigurations.dialog.back")}</Button> : null}
          <Button disabled={saveMutation.isPending} onClick={onClose} variant="outline">{t("agents.edit.cancel")}</Button>
          {stage === "provider" ? <Button data-testid="cli-config-provider-continue" disabled={!selectedPreset && !customSelected} onClick={() => setStage("configuration")}>{t("agentConfigurations.dialog.continue")}<ChevronRight className="h-4 w-4" /></Button> : <Button disabled={saveMutation.isPending || !name.trim()} onClick={() => void saveMutation.mutateAsync().catch(() => undefined)}>{saveMutation.isPending ? t("agents.edit.saving") : t("agents.edit.save")}</Button>}
        </div>
    </ApplicationDialog>
  );
}
