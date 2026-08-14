import { Trash2 } from "lucide-react";
import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { useTranslation } from "react-i18next";
import { Badge } from "../../../components/ui/badge";
import { Button } from "../../../components/ui/button";
import { useConfirmation } from "../../../components/ui/use-confirmation";
import { formatAppDateTime } from "../../../i18n/format";
import type { AgentService } from "../../../services/agent-service";
import { agentService as defaultAgentService } from "../../../services/runtime-agent-client";
import type { AgentMemory } from "../../../types/agent";
import { useSettings } from "../../settings-provider";
import { SectionPanel, SettingsRow } from "../page-parts";

/** `add-cli-memory-support`: memories are a single host-level pool shared by every agent — this
 * key is no longer scoped to a particular agent id. */
const memoriesQueryKey = ["agents", "memories"] as const;

function MemoryToggle({ ariaLabel, checked, disabled, onToggle }: { ariaLabel: string; checked: boolean; disabled: boolean; onToggle: () => void }) {
  return (
    <button
      aria-checked={checked}
      aria-label={ariaLabel}
      className={`relative h-6 w-11 shrink-0 rounded-full transition-colors ${checked ? "bg-primary" : "bg-muted-foreground/40"} disabled:opacity-40`}
      disabled={disabled}
      onClick={onToggle}
      role="switch"
      type="button"
    >
      <span className={`absolute left-1 top-1 h-4 w-4 rounded-full bg-background shadow-sm transition-transform ${checked ? "translate-x-5" : "translate-x-0"}`} />
    </button>
  );
}

export function AgentMemorySection({ service = defaultAgentService }: { service?: AgentService }) {
  const { i18n, t } = useTranslation();
  const { confirm, confirmationDialog } = useConfirmation();
  const queryClient = useQueryClient();
  const { loading, reportClientLogEvent, saveSetting, savingKey, settings } = useSettings();
  const settingsBusy = loading || savingKey !== null;
  const memoryEnabled = settings.memoryEnabled;

  const memoriesQuery = useQuery({
    queryKey: memoriesQueryKey,
    queryFn: () => service.listAllMemories(),
  });
  const memories = memoriesQuery.data ?? [];

  const deleteMutation = useMutation({
    mutationFn: (memoryId: string) => service.deleteAgentMemory(memoryId),
    onSuccess: () => queryClient.invalidateQueries({ queryKey: memoriesQueryKey }),
  });
  const resetMutation = useMutation({
    mutationFn: () => service.resetAllMemories(),
    onSuccess: () => queryClient.invalidateQueries({ queryKey: memoriesQueryKey }),
  });

  function toggleSetting(key: "memoryEnabled" | "memoryToolAssistedChatsEnabled", nextValue: boolean) {
    void saveSetting(key, nextValue).catch((cause) => {
      const message = cause instanceof Error ? cause.message : String(cause);
      void reportClientLogEvent({
        level: "error",
        kind: "critical-operation-failure",
        message,
        source: "AgentMemorySection.toggleSetting",
        details: { key },
      });
    });
  }

  async function handleDelete(memory: AgentMemory) {
    if (!(await confirm({ title: t("personalization.memory.confirmDelete"), tone: "danger" }))) return;
    deleteMutation.mutate(memory.id);
  }

  async function handleReset() {
    if (!(await confirm({ title: t("personalization.memory.confirmReset"), tone: "danger" }))) return;
    resetMutation.mutate();
  }

  const operationError = memoriesQuery.error ?? deleteMutation.error ?? resetMutation.error;

  return (
    <SectionPanel description={t("personalization.memory.description")} title={t("personalization.memory.title")} variant="settings">
      {confirmationDialog}
      <SettingsRow description={t("personalization.memory.enabledDesc")} title={t("personalization.memory.enabled")}>
        <MemoryToggle
          ariaLabel={t("personalization.memory.enabled")}
          checked={memoryEnabled}
          disabled={settingsBusy}
          onToggle={() => toggleSetting("memoryEnabled", !memoryEnabled)}
        />
      </SettingsRow>
      <SettingsRow description={t("personalization.memory.toolAssistedDesc")} title={t("personalization.memory.toolAssisted")}>
        <MemoryToggle
          ariaLabel={t("personalization.memory.toolAssisted")}
          checked={settings.memoryToolAssistedChatsEnabled}
          disabled={settingsBusy || !memoryEnabled}
          onToggle={() => toggleSetting("memoryToolAssistedChatsEnabled", !settings.memoryToolAssistedChatsEnabled)}
        />
      </SettingsRow>

      <div className="border-t border-border/70 px-5 py-4 sm:px-6">
        <div className="mb-3 flex items-center justify-between gap-3">
          <h4 className="text-sm font-semibold leading-5">{t("personalization.memory.listTitle")}</h4>
          <Button
            disabled={resetMutation.isPending || memories.length === 0}
            onClick={handleReset}
            size="sm"
            variant="outline"
          >
            <Trash2 className="h-3.5 w-3.5" aria-hidden="true" />
            {t("personalization.memory.reset")}
          </Button>
        </div>

        {memoriesQuery.isLoading ? (
          <p className="text-sm text-muted-foreground">{t("personalization.memory.loading")}</p>
        ) : memories.length === 0 ? (
          <p className="text-sm text-muted-foreground">{t("personalization.memory.empty")}</p>
        ) : (
          <ul className="grid gap-2">
            {memories.map((memory) => (
              <li className="ucd-panel rounded-md p-3" key={memory.id}>
                <div className="flex items-start justify-between gap-3">
                  <p className="min-w-0 flex-1 wrap-break-word text-sm">{memory.content}</p>
                  <Button
                    className="h-7 shrink-0 px-2 text-xs"
                    disabled={deleteMutation.isPending}
                    onClick={() => handleDelete(memory)}
                    type="button"
                    variant="outline"
                  >
                    <Trash2 className="h-3.5 w-3.5" aria-hidden="true" />
                    {t("personalization.memory.delete")}
                  </Button>
                </div>
                <div className="mt-2 flex flex-wrap items-center gap-2 text-xs text-muted-foreground">
                  <Badge tone={memory.source === "automatic" ? "muted" : "default"}>{t(`personalization.memory.source.${memory.source}`)}</Badge>
                  <Badge tone="muted">{memory.agentId}</Badge>
                  <span>{memory.folder ?? t("personalization.memory.folderGlobal")}</span>
                  <span>{formatAppDateTime(memory.createdAt, i18n.language, { dateStyle: "medium", timeStyle: "short" })}</span>
                </div>
              </li>
            ))}
          </ul>
        )}
        {operationError ? (
          <p className="mt-3 rounded-md border p-3 text-sm ucd-status-warning" role="alert">
            {operationError instanceof Error ? operationError.message : String(operationError)}
          </p>
        ) : null}
      </div>
    </SectionPanel>
  );
}
