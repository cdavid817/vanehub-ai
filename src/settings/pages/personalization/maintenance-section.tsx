import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { Wrench } from "lucide-react";
import { useState } from "react";
import { useTranslation } from "react-i18next";
import { Badge } from "../../../components/ui/badge";
import { Button } from "../../../components/ui/button";
import { formatAppDateTime } from "../../../i18n/format";
import type { AgentService } from "../../../services/agent-service";
import { agentService as defaultAgentService } from "../../../services/runtime-agent-client";
import type { MaintenanceResult } from "../../../types/personalization-memory";
import { SectionPanel } from "../page-parts";

export const healthQueryKey = ["personalization", "health"] as const;

/**
 * What state the store is in, and the one action that repairs it.
 *
 * A rebuild is how malformed entries, quarantined ones and a projection that disagrees with the
 * files become visible at all: nothing counts them until something walks the directory. So the
 * result of the last run is the diagnostic, and the panel says plainly when there has never been
 * one rather than showing zeros that were never measured.
 */
export function PersonalizationMaintenanceSection({
  service = defaultAgentService,
}: {
  service?: AgentService;
}) {
  const { t, i18n } = useTranslation();
  const queryClient = useQueryClient();
  const [result, setResult] = useState<MaintenanceResult | null>(null);
  const [failed, setFailed] = useState(false);

  const healthQuery = useQuery({
    queryKey: healthQueryKey,
    queryFn: () => service.getPersonalizationHealth(),
  });

  const rebuildMutation = useMutation({
    mutationFn: () => service.reconcilePersonalizationMemories(),
    onMutate: () => {
      setResult(null);
      setFailed(false);
    },
    onSuccess: (outcome) => {
      setResult(outcome);
      void queryClient.invalidateQueries({ queryKey: healthQueryKey });
      void queryClient.invalidateQueries({ queryKey: ["personalization", "memories"] });
    },
    onError: () => setFailed(true),
  });

  const health = healthQuery.data;

  return (
    <SectionPanel
      description={t("personalization.maintenance.description")}
      icon={Wrench}
      title={t("personalization.maintenance.title")}
    >
      {healthQuery.error ? (
        <p className="text-sm ucd-status-danger" data-testid="personalization-maintenance-error" role="alert">
          {t("personalization.maintenance.healthFailed")}
        </p>
      ) : !health ? (
        <p className="text-sm text-muted-foreground">{t("personalization.memory.loading")}</p>
      ) : (
        <div className="flex flex-col gap-4" data-testid="personalization-maintenance">
          <div className="flex flex-wrap items-center gap-2">
            <Badge tone={health.memoryAvailable ? "success" : "warning"} data-testid="personalization-maintenance-state">
              {t(`personalization.maintenance.state.${health.state}`)}
            </Badge>
            {health.repairRequired ? (
              <Badge tone="danger" data-testid="personalization-maintenance-repair">
                {t("personalization.maintenance.repairRequired")}
              </Badge>
            ) : null}
            <span className="text-xs text-muted-foreground" data-testid="personalization-maintenance-pending">
              {t("personalization.maintenance.pending", { count: health.pendingCandidates })}
            </span>
          </div>

          <p className="text-sm text-muted-foreground" data-testid="personalization-maintenance-last-run">
            {/* Never-run and ran-and-found-nothing are different answers, and a screen that showed
                the same thing for both would send the user re-running a rebuild blindly. */}
            {health.lastReconciledAt
              ? t("personalization.maintenance.lastRun", {
                  when: formatAppDateTime(health.lastReconciledAt, i18n.language, {
                    dateStyle: "medium",
                    timeStyle: "short",
                  }),
                })
              : t("personalization.maintenance.neverRun")}
          </p>

          {result ? <RebuildResult result={result} /> : null}
          {failed ? (
            <p className="text-sm ucd-status-danger" data-testid="personalization-maintenance-failed" role="alert">
              {t("personalization.maintenance.rebuildFailed")}
            </p>
          ) : null}

          <div>
            <Button
              data-testid="personalization-maintenance-rebuild"
              disabled={rebuildMutation.isPending}
              onClick={() => rebuildMutation.mutate()}
            >
              {rebuildMutation.isPending
                ? t("personalization.maintenance.rebuilding")
                : t("personalization.maintenance.rebuild")}
            </Button>
          </div>
        </div>
      )}
    </SectionPanel>
  );
}

function RebuildResult({ result }: { result: MaintenanceResult }) {
  const { t } = useTranslation();
  return (
    <div className="rounded-md border border-border/70 p-3" data-testid="personalization-maintenance-result">
      <dl className="grid gap-2 sm:grid-cols-2">
        <Entry label={t("personalization.maintenance.result.scanned")} value={result.matched} />
        <Entry label={t("personalization.maintenance.result.projection")} value={result.removedProjectionRows} />
        <Entry label={t("personalization.maintenance.result.retrieval")} value={result.revokedRetrievalEntries} />
        <Entry label={t("personalization.maintenance.result.quarantined")} value={result.quarantined} />
      </dl>
      {result.failures.length > 0 ? (
        <div className="mt-3 rounded-md border p-2 text-xs ucd-status-warning" data-testid="personalization-maintenance-partial" role="alert">
          <p>{t("personalization.maintenance.result.partial")}</p>
          <ul className="mt-1 list-disc pl-5">
            {result.failures.map((phase) => (
              <li key={phase}>{t(`personalization.reset.phase.${phase}`)}</li>
            ))}
          </ul>
        </div>
      ) : (
        <p className="mt-2 text-xs" data-testid="personalization-maintenance-clean">
          {t("personalization.maintenance.result.clean")}
        </p>
      )}
    </div>
  );
}

function Entry({ label, value }: { label: string; value: number }) {
  return (
    <div className="min-w-0">
      <dt className="text-xs text-muted-foreground">{label}</dt>
      <dd className="text-sm font-medium">{value}</dd>
    </div>
  );
}
