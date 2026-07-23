import { Activity, Database, RadioTower, Save, ShieldCheck } from "lucide-react";
import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { useEffect, useMemo, useState } from "react";
import { useTranslation } from "react-i18next";
import { Badge } from "../../components/ui/badge";
import { Button } from "../../components/ui/button";
import type { ExecutionObservabilityService } from "../../services/execution-observability-service";
import { executionObservabilityService } from "../../services/runtime-execution-observability-client";
import type { ObservabilitySettings } from "../../types/execution-observability";
import { PageHeader, SectionPanel } from "./page-parts";

const queryKey = ["execution-observability"] as const;

function supportedDraft(settings: ObservabilitySettings): ObservabilitySettings {
  return { ...settings, otlpAuthToken: null };
}

export function validateObservabilitySettings(input: ObservabilitySettings) {
  const errors: Partial<Record<"retentionDays" | "samplingRatio" | "otlpEndpoint", string>> = {};
  if (!Number.isInteger(input.retentionDays) || input.retentionDays < 1 || input.retentionDays > 90) {
    errors.retentionDays = "observability.validation.retention";
  }
  if (!Number.isFinite(input.samplingRatio) || input.samplingRatio < 0 || input.samplingRatio > 1) {
    errors.samplingRatio = "observability.validation.sampling";
  }
  if (input.otlpEnabled) {
    try {
      const endpoint = new URL(input.otlpEndpoint ?? "");
      if (!["http:", "https:"].includes(endpoint.protocol) || endpoint.username || endpoint.password || endpoint.hash) {
        errors.otlpEndpoint = "observability.validation.endpoint";
      }
    } catch {
      errors.otlpEndpoint = "observability.validation.endpoint";
    }
  }
  return errors;
}

export function ObservabilitySettingsPage({
  searchTerm = "",
  service = executionObservabilityService,
}: {
  searchTerm?: string;
  service?: ExecutionObservabilityService;
}) {
  const { t } = useTranslation();
  const queryClient = useQueryClient();
  const settingsQuery = useQuery({ queryKey: [...queryKey, "settings"], queryFn: () => service.getSettings() });
  const capabilitiesQuery = useQuery({ queryKey: [...queryKey, "capabilities"], queryFn: () => service.getObservationCapabilities() });
  const [draft, setDraft] = useState<ObservabilitySettings | null>(settingsQuery.data ? supportedDraft(settingsQuery.data) : null);
  const [notice, setNotice] = useState<string | null>(null);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    if (settingsQuery.data) setDraft(supportedDraft(settingsQuery.data));
  }, [settingsQuery.data]);

  const mutation = useMutation({
    mutationFn: (settings: ObservabilitySettings) => service.updateSettings(settings),
    onSuccess: async (saved) => {
      setDraft(supportedDraft(saved));
      setNotice(t("observability.saved"));
      setError(null);
      queryClient.setQueryData([...queryKey, "settings"], saved);
    },
    onError: (reason) => {
      setNotice(null);
      setError(reason instanceof Error ? reason.message : String(reason));
    },
  });
  const validation = useMemo(
    () => (draft ? validateObservabilitySettings(draft) : {}),
    [draft],
  );
  const relayAvailable = (capabilitiesQuery.data ?? []).some((item) => item.relaySupported);
  const visible = !searchTerm.trim() || [
    t("observability.title"),
    t("observability.local.title"),
    t("observability.export.title"),
    t("observability.capture.title"),
    t("observability.mcp.title"),
  ].some((value) => value.toLowerCase().includes(searchTerm.trim().toLowerCase()));

  function update<K extends keyof ObservabilitySettings>(key: K, value: ObservabilitySettings[K]) {
    setDraft((current) => (current ? { ...current, [key]: value } : current));
    setNotice(null);
  }

  function save() {
    if (!draft || Object.keys(validation).length) return;
    mutation.mutate(draft);
  }

  if (settingsQuery.isLoading || !draft) {
    return <div className="text-sm text-muted-foreground">{t("observability.loading")}</div>;
  }

  if (!visible) {
    return <div className="text-sm text-muted-foreground">{t("observability.searchEmpty")}</div>;
  }

  return (
    <div className="space-y-4">
      <PageHeader
        actions={
          <Button disabled={mutation.isPending || Object.keys(validation).length > 0} onClick={save}>
            <Save className="h-4 w-4" aria-hidden="true" />
            {mutation.isPending ? t("observability.saving") : t("observability.save")}
          </Button>
        }
        description={t("observability.description")}
        icon={Activity}
        title={t("observability.title")}
      />
      {settingsQuery.isError ? <div className="rounded border p-3 text-sm ucd-status-danger">{t("observability.loadError")}</div> : null}
      {error ? <div className="rounded border p-3 text-sm ucd-status-danger">{error}</div> : null}
      {notice ? <div className="rounded border p-3 text-sm ucd-status-success">{notice}</div> : null}

      <SectionPanel description={t("observability.local.description")} icon={Database} title={t("observability.local.title")}>
        <ToggleRow checked={draft.localTimelineEnabled} label={t("observability.local.enabled")} onChange={(value) => update("localTimelineEnabled", value)} />
        <NumberField errorKey={validation.retentionDays} label={t("observability.retention")} max={90} min={1} onChange={(value) => update("retentionDays", value)} value={draft.retentionDays} />
        <p className="mt-2 text-xs text-muted-foreground">{t("observability.retentionHint")}</p>
      </SectionPanel>

      <SectionPanel description={t("observability.export.description")} icon={RadioTower} title={t("observability.export.title")}>
        <div className="mb-4 rounded border p-3 text-sm text-muted-foreground">{t("observability.export.restart")}</div>
        <ToggleRow checked={draft.otlpEnabled} label={t("observability.export.enabled")} onChange={(value) => update("otlpEnabled", value)} />
        <div className="mt-4 grid gap-4 md:grid-cols-2">
          <label className="grid gap-1.5 text-sm">
            <span className="font-medium text-muted-foreground">{t("observability.export.endpoint")}</span>
            <input className="ucd-input h-9 rounded px-3 font-mono text-sm" onChange={(event) => update("otlpEndpoint", event.target.value || null)} placeholder={t("observability.export.endpointPlaceholder")} value={draft.otlpEndpoint ?? ""} />
          </label>
          <NumberField errorKey={validation.samplingRatio} label={t("observability.export.sampling")} max={1} min={0} onChange={(value) => update("samplingRatio", value)} step={0.05} value={draft.samplingRatio} />
        </div>
        <label className="mt-4 grid gap-1.5 text-sm">
          <span className="font-medium text-muted-foreground">{t("observability.export.auth")}</span>
          <input autoComplete="new-password" className="ucd-input h-9 rounded px-3 font-mono text-sm" onChange={(event) => update("otlpAuthToken", event.target.value)} placeholder={draft.otlpAuthConfigured ? t("observability.export.authConfigured") : t("observability.export.authPlaceholder")} type="password" value={draft.otlpAuthToken ?? ""} />
        </label>
        <p className="mt-2 text-xs text-muted-foreground">{t("observability.export.samplingHint")}</p>
      </SectionPanel>

      <SectionPanel description={t("observability.capture.description")} icon={ShieldCheck} title={t("observability.capture.title")}>
        <label className="grid gap-1.5 text-sm">
          <span className="font-medium text-muted-foreground">{t("observability.capture.policy")}</span>
          <select className="ucd-input h-9 rounded px-3 text-sm" onChange={(event) => update("capturePolicy", event.target.value as ObservabilitySettings["capturePolicy"])} value={draft.capturePolicy}>
            <option value="metadata_only">{t("observability.capture.metadata")}</option>
            <option value="redacted_content">{t("observability.capture.redacted")}</option>
          </select>
        </label>
        <div className={`mt-3 rounded border p-3 text-sm ${draft.capturePolicy === "redacted_content" ? "ucd-status-warning" : "text-muted-foreground"}`}>
          {t(draft.capturePolicy === "redacted_content" ? "observability.capture.warning" : "observability.capture.safeDefault")}
        </div>
      </SectionPanel>

      <SectionPanel description={t("observability.mcp.description")} title={t("observability.mcp.title")}>
        <ToggleRow checked={draft.mcpRelayEnabled} disabled={!relayAvailable} label={t("observability.mcp.relay")} onChange={(value) => update("mcpRelayEnabled", value)} />
        <p className="mt-2 text-xs text-muted-foreground">{t(relayAvailable ? "observability.mcp.available" : "observability.mcp.unavailable")}</p>
        <div className="mt-4 grid gap-2 sm:grid-cols-2 xl:grid-cols-4">
          {(capabilitiesQuery.data ?? []).filter((item) => item.transport === "stdio").map((item) => (
            <div className="ucd-panel rounded p-3" key={item.agentId}>
              <div className="font-mono text-sm font-medium">{item.agentId}</div>
              <Badge className="mt-2" tone={item.relaySupported ? "warning" : "muted"}>{t(item.relaySupported ? "observability.capability.relay" : "observability.capability.opaque")}</Badge>
            </div>
          ))}
        </div>
      </SectionPanel>
    </div>
  );
}

function ToggleRow({ checked, disabled = false, label, onChange }: { checked: boolean; disabled?: boolean; label: string; onChange: (value: boolean) => void }) {
  return (
    <label className="flex items-center justify-between gap-4 py-2 text-sm">
      <span className="font-medium">{label}</span>
      <input checked={checked} className="h-4 w-4 accent-primary" disabled={disabled} onChange={(event) => onChange(event.target.checked)} type="checkbox" />
    </label>
  );
}

function NumberField({ disabled = false, errorKey, label, max, min, onChange, step = 1, value }: { disabled?: boolean; errorKey?: string; label: string; max: number; min: number; onChange: (value: number) => void; step?: number; value: number }) {
  const { t } = useTranslation();
  return (
    <label className="grid gap-1.5 text-sm">
      <span className="font-medium text-muted-foreground">{label}</span>
      <input className="ucd-input h-9 rounded px-3 text-sm" disabled={disabled} max={max} min={min} onChange={(event) => onChange(event.target.valueAsNumber)} step={step} type="number" value={value} />
      {errorKey ? <span className="text-xs text-destructive">{t(errorKey)}</span> : null}
    </label>
  );
}
