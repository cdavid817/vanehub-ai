import { useEffect, useState } from "react";
import { useMutation, useQuery } from "@tanstack/react-query";
import { useTranslation } from "react-i18next";
import { Button } from "../../../components/ui/button";
import type { AgentService } from "../../../services/agent-service";
import { agentService as defaultAgentService } from "../../../services/runtime-agent-client";
import type { AgentRegistryEntry, UpdateApiAgentInput } from "../../../types/agent";

export function AgentEditDialog({
  agent,
  onClose,
  onSaved,
  service = defaultAgentService,
}: {
  agent: AgentRegistryEntry;
  onClose: () => void;
  onSaved: (agent: AgentRegistryEntry) => void;
  service?: AgentService;
}) {
  const { t } = useTranslation();
  const [displayName, setDisplayName] = useState(agent.displayName);
  const [modelId, setModelId] = useState("");
  const [baseUrl, setBaseUrl] = useState("");
  const [newApiKey, setNewApiKey] = useState("");
  const [error, setError] = useState<string | null>(null);

  const configQuery = useQuery({
    queryKey: ["agents", "provider-config", agent.id],
    queryFn: () => service.getApiAgentProviderConfig(agent.id),
  });

  useEffect(() => {
    if (configQuery.data) {
      setModelId(configQuery.data.modelId);
      setBaseUrl(configQuery.data.baseUrl ?? "");
    }
  }, [configQuery.data]);

  const updateMutation = useMutation({
    mutationFn: (input: UpdateApiAgentInput) => service.updateApiAgent(agent.id, input),
    onSuccess: (updated) => onSaved(updated),
    onError: (err) => setError(err instanceof Error ? err.message : String(err)),
  });

  function handleSave() {
    setError(null);
    if (!displayName.trim() || !modelId.trim()) {
      setError(t("agents.registerApiAgent.errors.incomplete"));
      return;
    }
    const input: UpdateApiAgentInput = {
      displayName: displayName.trim(),
      modelId: modelId.trim(),
      baseUrl: baseUrl.trim() || null,
      newApiKey: newApiKey.trim() || null,
    };
    void updateMutation.mutateAsync(input).catch(() => undefined);
  }

  return (
    <div className="fixed inset-0 z-50 flex items-center justify-center bg-black/40 p-4">
      <section className="w-full max-w-md rounded-lg bg-background p-5 shadow-xl">
        <div className="mb-4 flex items-center justify-between">
          <h3 className="text-lg font-semibold">{t("agents.edit.title", { agent: agent.displayName })}</h3>
          <button className="text-sm text-muted-foreground" onClick={onClose} type="button">
            {t("agents.edit.close")}
          </button>
        </div>

        {configQuery.isLoading ? (
          <p className="text-sm text-muted-foreground">{t("agents.edit.loading")}</p>
        ) : (
          <div className="grid gap-3">
            <label className="flex flex-col gap-1 text-sm">
              {t("agents.registerApiAgent.displayName")}
              <input
                className="ucd-input h-9 rounded px-3 text-sm outline-hidden focus-visible:ring-2 focus-visible:ring-ring"
                onChange={(event) => setDisplayName(event.target.value)}
                value={displayName}
              />
            </label>
            <label className="flex flex-col gap-1 text-sm">
              {t("agents.registerApiAgent.modelId")}
              <input
                className="ucd-input h-9 rounded px-3 text-sm outline-hidden focus-visible:ring-2 focus-visible:ring-ring"
                onChange={(event) => setModelId(event.target.value)}
                value={modelId}
              />
            </label>
            <label className="flex flex-col gap-1 text-sm">
              {t("agents.registerApiAgent.baseUrl")}
              <input
                className="ucd-input h-9 rounded px-3 text-sm outline-hidden focus-visible:ring-2 focus-visible:ring-ring"
                onChange={(event) => setBaseUrl(event.target.value)}
                placeholder={t("agents.edit.baseUrlHint")}
                value={baseUrl}
              />
            </label>
            <label className="flex flex-col gap-1 text-sm">
              {t("agents.edit.newApiKey")}
              <input
                autoComplete="off"
                className="ucd-input h-9 rounded px-3 text-sm outline-hidden focus-visible:ring-2 focus-visible:ring-ring"
                onChange={(event) => setNewApiKey(event.target.value)}
                placeholder={t("agents.edit.newApiKeyHint")}
                type="password"
                value={newApiKey}
              />
            </label>
          </div>
        )}

        {error ? <p className="mt-3 text-sm ucd-status-warning">{error}</p> : null}

        <div className="mt-4 flex justify-end gap-2">
          <Button onClick={onClose} variant="outline">
            {t("agents.edit.cancel")}
          </Button>
          <Button disabled={updateMutation.isPending || configQuery.isLoading} onClick={handleSave}>
            {updateMutation.isPending ? t("agents.edit.saving") : t("agents.edit.save")}
          </Button>
        </div>
      </section>
    </div>
  );
}
