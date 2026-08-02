import { ChevronDown, Clock3, FileJson2, RefreshCw, ShieldCheck } from "lucide-react";
import { useTranslation } from "react-i18next";
import { Badge } from "../../../components/ui/badge";
import type { CliConfigStatus } from "../../../types/cli-agent-config";

export function CliConfigStatusSummary({ status }: { status: CliConfigStatus }) {
  const { t } = useTranslation();
  const tone = status.driftState === "applied" ? "success" : status.driftState === "drifted" || status.driftState === "malformed" ? "warning" : "muted";

  return (
    <section className="rounded-lg border border-border bg-muted/35 px-4 py-3" aria-label={t("agentConfigurations.status.title")}>
      <div className="flex flex-wrap items-center gap-x-4 gap-y-2">
        <div className="flex items-center gap-2 text-sm font-semibold"><ShieldCheck className="h-4 w-4 text-primary" />{t("agentConfigurations.status.title")}</div>
        <div className="flex flex-wrap gap-2"><Badge tone={tone}>{t(`agents.globalConfig.drift.${status.driftState}`)}</Badge>{status.simulated ? <Badge tone="muted">{t("agents.globalConfig.webSimulation")}</Badge> : null}</div>
        <p className="flex items-center gap-1.5 text-xs text-muted-foreground sm:ml-auto"><Clock3 className="h-3.5 w-3.5" />{t("agentConfigurations.status.lastApplied")}: {status.lastAppliedAt ? new Date(status.lastAppliedAt).toLocaleString() : t("agentConfigurations.status.neverApplied")}</p>
      </div>
      <div className="mt-2 flex flex-wrap items-center gap-2 text-xs text-muted-foreground">
        <RefreshCw className="h-3.5 w-3.5" />
        <span>{t(`agentConfigurations.status.startupSync.${status.startupSync.state}`, {
          imported: status.startupSync.imported,
          updated: status.startupSync.updated,
          skipped: status.startupSync.skipped,
        })}</span>
      </div>
      {status.startupSync.warnings.map((warning) => <p className="mt-1 text-xs ucd-status-warning" key={warning}>{warning}</p>)}
      <details className="group mt-2 text-xs text-muted-foreground">
        <summary className="flex w-fit cursor-pointer list-none items-center gap-1.5 hover:text-foreground"><FileJson2 className="h-3.5 w-3.5" />{t("agentConfigurations.status.paths")} ({status.resolvedPaths.length})<ChevronDown className="h-3.5 w-3.5 transition-transform group-open:rotate-180" /></summary>
        <div className="mt-2 space-y-1 pl-5">{status.resolvedPaths.length ? status.resolvedPaths.map((path) => <p className="break-all font-mono" key={path}>{path}</p>) : <p>{t("agentConfigurations.status.noPaths")}</p>}</div>
      </details>
    </section>
  );
}
