import { useQuery } from "@tanstack/react-query";
import { Activity, LoaderCircle, RefreshCw } from "lucide-react";
import { useTranslation } from "react-i18next";
import { Button } from "../../../components/ui/button";
import type { AgentService } from "../../../services/agent-service";
import { agentService as defaultAgentService } from "../../../services/runtime-agent-client";
import { SectionPanel } from "../page-parts";
import { lspServerStatusQueryKey } from "./lsp-configuration-section";
import { LspRuntimeStatusCard } from "./lsp-runtime-status-card";

export const lspStatusPollingIntervalMs = 5_000;

export function LspRuntimeStatusPanel({
  service = defaultAgentService,
}: {
  service?: AgentService;
}) {
  const { t } = useTranslation();
  const statusQuery = useQuery({
    queryKey: lspServerStatusQueryKey,
    queryFn: () => service.getLspServerStatus(),
    refetchInterval: lspStatusPollingIntervalMs,
    refetchIntervalInBackground: false,
  });

  return (
    <SectionPanel
      description={t("lspSettings.runtime.description")}
      icon={Activity}
      title={t("lspSettings.runtime.title")}
      variant="settings"
    >
      <div className="space-y-4 p-5 sm:p-6">
        <div className="flex justify-end">
          <Button
            disabled={statusQuery.isFetching}
            onClick={() => { void statusQuery.refetch(); }}
            size="sm"
            type="button"
            variant="outline"
          >
            <RefreshCw className={statusQuery.isFetching ? "animate-spin" : ""} aria-hidden="true" />
            {t("lspSettings.runtime.refresh")}
          </Button>
        </div>

        {statusQuery.isLoading ? (
          <div className="flex min-h-24 items-center justify-center gap-2 text-sm text-muted-foreground" role="status">
            <LoaderCircle className="h-4 w-4 animate-spin" aria-hidden="true" />
            {t("lspSettings.loading")}
          </div>
        ) : null}
        {!statusQuery.isLoading && statusQuery.error ? (
          <div>
            <p className="rounded-md border p-3 text-sm ucd-status-warning" role="alert">
              {t("lspSettings.loadError")}
            </p>
            <Button className="mt-3" onClick={() => { void statusQuery.refetch(); }} size="sm" type="button" variant="outline">
              {t("lspSettings.retry")}
            </Button>
          </div>
        ) : null}
        {!statusQuery.isLoading && !statusQuery.error && statusQuery.data?.length === 0 ? (
          <p className="rounded-md border border-dashed border-border p-4 text-sm text-muted-foreground">
            {t("lspSettings.runtime.empty")}
          </p>
        ) : null}
        {statusQuery.data && statusQuery.data.length > 0 ? (
          <div aria-atomic="false" aria-live="polite" className="grid gap-4">
            {statusQuery.data.map((status) => (
              <LspRuntimeStatusCard
                key={`${status.language}:${status.server}:${status.relativeProjectRoot}`}
                status={status}
              />
            ))}
          </div>
        ) : null}

        <p className="border-t border-border/70 pt-4 text-xs leading-5 text-muted-foreground">
          {t("lspSettings.runtime.unsupportedMetrics")}
        </p>
      </div>
    </SectionPanel>
  );
}
