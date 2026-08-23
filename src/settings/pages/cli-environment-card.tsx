import {
  AlertTriangle,
  ArrowUpCircle,
  CheckCircle2,
  ChevronDown,
  ChevronRight,
  RefreshCw,
  Stethoscope,
} from "lucide-react";
import { useTranslation } from "react-i18next";
import { AgentBrandIcon } from "../../components/agent-brand-icon";
import { Badge } from "../../components/ui/badge";
import { Button } from "../../components/ui/button";
import { getAgentVisualIdentity } from "../../lib/agent-visual-identity";
import { normalizeDisplayPath } from "../../lib/session-path";
import type { CliEnvironmentSnapshot } from "../../types/cli-environment-snapshot";
import type { OperationTask } from "../../types/operation";
import { canRequestChange, installedVersion, recommendedSourceId, targetVersionOptions } from "./cli-action-selection";
import { CliInstallationList } from "./cli-installation-list";

interface CliEnvironmentCardProps {
  snapshot: CliEnvironmentSnapshot;
  selectedVersion: string;
  operation?: OperationTask;
  diagnosticsExpanded: boolean;
  operationExpanded: boolean;
  refreshing: boolean;
  mutating: boolean;
  onSelectedVersionChange: (version: string) => void;
  onRefresh: () => void;
  onRequestChange: (targetVersion: string) => void;
  onToggleDiagnostics: () => void;
  onToggleOperation: () => void;
}

/** The badge tone follows the backend's own overall state; nothing is re-derived here. */
function statusTone(snapshot: CliEnvironmentSnapshot): "success" | "warning" | "muted" {
  if (snapshot.conflicts.length > 0) return "warning";
  if (snapshot.overallState === "ready" || snapshot.overallState === "up-to-date") return "success";
  if (snapshot.overallState === "missing" || snapshot.overallState === "broken") return "warning";
  return "muted";
}

export function CliEnvironmentCard(props: CliEnvironmentCardProps) {
  const { t } = useTranslation();
  const { snapshot, operation } = props;
  const options = targetVersionOptions(snapshot);
  const installed = installedVersion(snapshot);
  const sourceId = recommendedSourceId(snapshot);
  const operationRunning = operation?.status === "running" || operation?.status === "queued";
  // What the select actually shows. With no stored choice the browser displays the first option,
  // so the button must act on that one -- not on "nothing", which would let it offer a change to a
  // version the user can see is already installed.
  const selected = props.selectedVersion || options[0] || null;
  const changeable = canRequestChange(snapshot, selected);
  const identity = getAgentVisualIdentity(snapshot.agentId);
  const pathSelected = snapshot.installations.find(
    (installation) => installation.id === snapshot.pathSelectedInstallationId,
  );
  // The one conflict the user must act on first. Its code is localized; nothing parses a message.
  const blocking = snapshot.conflicts.find((conflict) => conflict.blocksMutation);

  return (
    <section className="ucd-panel ucd-interactive flex min-h-72 flex-col rounded-lg p-4" data-cli-agent={snapshot.agentId}>
      <div className="flex items-start justify-between gap-3">
        <div className="flex min-w-0 items-start gap-3">
          <span className={`flex h-9 w-9 shrink-0 items-center justify-center rounded-xl border ${identity.tone}`}>
            <AgentBrandIcon agentId={snapshot.agentId} className="h-5 w-5" />
          </span>
          <div className="min-w-0">
            <h3 className="truncate font-semibold">{snapshot.displayName}</h3>
            <p className="mt-1 truncate text-xs text-muted-foreground">{snapshot.provider}</p>
          </div>
        </div>
        <Button
          aria-label={t("cli.refreshOne", { name: snapshot.displayName })}
          disabled={props.refreshing}
          size="icon"
          title={t("cli.refreshOne", { name: snapshot.displayName })}
          variant="ghost"
          onClick={props.onRefresh}
        >
          <RefreshCw className={props.refreshing ? "animate-spin" : ""} aria-hidden="true" />
        </Button>
      </div>

      <div className="mt-3 flex flex-wrap gap-2">
        <Badge tone={statusTone(snapshot)}>{t(`cli.overallState.${snapshot.overallState}`)}</Badge>
        {snapshot.freshness === "stale" ? <Badge tone="warning">{t("cli.freshness.stale")}</Badge> : null}
        {sourceId ? <Badge tone="muted">{t(`cli.source.${sourceId}`)}</Badge> : null}
        {snapshot.installations.length > 1 ? (
          <Badge tone="warning">{t("cli.installationsCount", { count: snapshot.installations.length })}</Badge>
        ) : null}
      </div>

      <dl className="mt-4 grid gap-3 text-sm sm:grid-cols-2">
        <div>
          <dt className="text-xs text-muted-foreground">{t("cli.currentVersion")}</dt>
          <dd className="mt-1 font-mono font-medium">{installed ?? t("cli.versionUnknown")}</dd>
        </div>
        <div>
          <dt className="text-xs text-muted-foreground">{t("cli.updateState")}</dt>
          <dd className="mt-1 font-medium">{t(`cli.update.${snapshot.update}`)}</dd>
        </div>
        <div className="sm:col-span-2">
          <dt className="text-xs text-muted-foreground">{t("cli.activePath")}</dt>
          <dd className="mt-1 break-all font-mono text-xs">
            {pathSelected ? normalizeDisplayPath(pathSelected.executablePath) : t("cli.notAvailable")}
          </dd>
        </div>
      </dl>

      {blocking ? (
        <div className="mt-4 flex gap-2 rounded-md border p-3 text-xs ucd-status-warning">
          <AlertTriangle className="mt-0.5 h-4 w-4 shrink-0" aria-hidden="true" />
          <span>{t(`cli.conflict.${blocking.reasonCode}`)}</span>
        </div>
      ) : null}

      <div className="mt-4 flex flex-wrap items-center gap-2">
        {options.length > 0 ? (
          <select
            aria-label={t("cli.targetVersion", { name: snapshot.displayName })}
            className="ucd-input h-9 min-w-36 flex-1 rounded px-3 text-sm outline-hidden focus-visible:ring-2 focus-visible:ring-ring"
            disabled={props.mutating || operationRunning}
            value={selected ?? ""}
            onChange={(event) => props.onSelectedVersionChange(event.target.value)}
          >
            {options.map((version) => <option key={version} value={version}>{version}</option>)}
          </select>
        ) : null}
        {changeable ? (
          <Button
            disabled={props.mutating || operationRunning}
            onClick={() => selected && props.onRequestChange(selected)}
          >
            <ArrowUpCircle aria-hidden="true" />
            {t("cli.action.change")}
          </Button>
        ) : selected !== null && selected === installed ? (
          // Already there. No button at all, so there is nothing to click that would do nothing.
          <span className="inline-flex h-9 items-center gap-2 text-xs text-muted-foreground">
            <CheckCircle2 className="h-4 w-4 text-[hsl(var(--success))]" aria-hidden="true" />
            {t("cli.action.current")}
          </span>
        ) : null}
      </div>

      <div className="mt-auto pt-4">
        <button className="flex w-full items-center justify-between border-t border-border pt-3 text-left text-xs font-medium" type="button" onClick={props.onToggleDiagnostics}>
          <span className="flex items-center gap-2"><Stethoscope className="h-3.5 w-3.5" />{t("cli.diagnostics.title")}</span>
          {props.diagnosticsExpanded ? <ChevronDown className="h-4 w-4" /> : <ChevronRight className="h-4 w-4" />}
        </button>
        {props.diagnosticsExpanded ? <div className="mt-3"><CliInstallationList installations={snapshot.installations} /></div> : null}

        {operation ? (
          <div className="mt-3 rounded-md border border-border p-3 text-xs">
            <button className="flex w-full items-center justify-between gap-3 text-left" type="button" onClick={props.onToggleOperation}>
              <span>{t("cli.operation")}: {t(`cli.operationStatus.${operation.status}`)}</span>
              {props.operationExpanded ? <ChevronDown className="h-4 w-4" /> : <ChevronRight className="h-4 w-4" />}
            </button>
            {props.operationExpanded ? (
              <div className="mt-3 max-h-40 overflow-auto rounded border border-border bg-[hsl(var(--panel-muted))] p-2 font-mono">
                {operation.logs.length === 0 ? <div>{t("cli.noLogs")}</div> : null}
                {operation.logs.map((log, index) => <div className="whitespace-pre-wrap" key={`${log.timestamp}-${index}`}>{log.line}</div>)}
                {operation.error ? <div className="mt-2 text-[hsl(var(--danger))]">{operation.error}</div> : null}
              </div>
            ) : null}
          </div>
        ) : null}
      </div>
    </section>
  );
}
