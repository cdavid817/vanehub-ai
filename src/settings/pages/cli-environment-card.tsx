import {
  AlertTriangle,
  ArrowUpCircle,
  CheckCircle2,
  ChevronDown,
  ChevronRight,
  Download,
  RefreshCw,
  Stethoscope,
} from "lucide-react";
import { useTranslation } from "react-i18next";
import { AgentBrandIcon } from "../../components/agent-brand-icon";
import { Badge } from "../../components/ui/badge";
import { Button } from "../../components/ui/button";
import { getAgentVisualIdentity } from "../../lib/agent-visual-identity";
import type { CliToolStatus } from "../../types/agent";
import type { OperationTask } from "../../types/operation";
import { deriveCliLifecycleGuidance, isManagedCliLifecycle, type CliVersionAction } from "./cli-management-utils";
import { CliInstallationList } from "./cli-installation-list";

interface CliEnvironmentCardProps {
  tool: CliToolStatus;
  selectedVersion: string;
  action: CliVersionAction;
  operation?: OperationTask;
  diagnosticsExpanded: boolean;
  operationExpanded: boolean;
  refreshing: boolean;
  packageBusy: boolean;
  onSelectedVersionChange: (version: string) => void;
  onRefresh: () => void;
  onRunAction: () => void;
  onToggleDiagnostics: () => void;
  onToggleOperation: () => void;
}

function statusTone(tool: CliToolStatus): "success" | "warning" | "muted" {
  if (tool.installed === true && isManagedCliLifecycle(tool.lifecycleEligibility) && tool.conflictState === "none") return "success";
  if (tool.installed === false || tool.lifecycleEligibility === "manual" || tool.conflictState !== "none") return "warning";
  return "muted";
}

function statusLabelKey(tool: CliToolStatus) {
  if (tool.installed === false) return "cli.status.missing";
  if (tool.installed === true && tool.installations.some((installation) => installation.isActive && !installation.runnable)) {
    return "cli.status.broken";
  }
  if (tool.conflictState !== "none") return "cli.status.conflict";
  if (tool.installed === true) return "cli.status.installed";
  if (tool.versionCheckStatus === "unsupported") return "cli.status.unsupported";
  return "cli.status.undetected";
}

function versionOptions(tool: CliToolStatus) {
  const options = [...tool.availableVersions];
  if (tool.latestVersion && !options.includes(tool.latestVersion)) options.unshift(tool.latestVersion);
  if (tool.currentVersion && !options.includes(tool.currentVersion)) options.push(tool.currentVersion);
  return [...new Set(options)];
}

export function CliEnvironmentCard(props: CliEnvironmentCardProps) {
  const { t } = useTranslation();
  const { tool, operation } = props;
  const options = versionOptions(tool);
  const activeInstallation = tool.installations.find((installation) => installation.isActive);
  const operationRunning = operation?.status === "running" || operation?.status === "queued";
  const canMutate = ["install", "upgrade", "downgrade"].includes(props.action);
  const showsDisabledUpgrade = tool.installed === true && !canMutate;
  const canRunPackageAction = canMutate && isManagedCliLifecycle(tool.lifecycleEligibility);
  const showsManualUpgrade = (props.action === "upgrade" || showsDisabledUpgrade) && !isManagedCliLifecycle(tool.lifecycleEligibility);
  const guidance = deriveCliLifecycleGuidance(tool);
  const identity = getAgentVisualIdentity(tool.agentId);
  const guidanceText = guidance?.kind === "source-native"
    ? t(guidance.key, { source: t(`cli.source.${guidance.source}`) })
    : guidance
      ? t(guidance.key)
      : null;

  return (
    <section className="ucd-panel ucd-interactive flex min-h-72 flex-col rounded-lg p-4" data-cli-agent={tool.agentId}>
      <div className="flex items-start justify-between gap-3">
        <div className="flex min-w-0 items-start gap-3">
          <span className={`flex h-9 w-9 shrink-0 items-center justify-center rounded-xl border ${identity.tone}`}>
            <AgentBrandIcon agentId={tool.agentId} className="h-5 w-5" />
          </span>
          <div className="min-w-0">
            <h3 className="truncate font-semibold">{tool.displayName}</h3>
            <p className="mt-1 truncate text-xs text-muted-foreground">{tool.packageName}</p>
          </div>
        </div>
        <Button
          aria-label={t("cli.refreshOne", { name: tool.displayName })}
          disabled={props.refreshing}
          size="icon"
          title={t("cli.refreshOne", { name: tool.displayName })}
          variant="ghost"
          onClick={props.onRefresh}
        >
          <RefreshCw className={props.refreshing ? "animate-spin" : ""} aria-hidden="true" />
        </Button>
      </div>

      <div className="mt-3 flex flex-wrap gap-2">
        <Badge tone={statusTone(tool)}>{t(statusLabelKey(tool))}</Badge>
        <Badge tone="muted">{t(`cli.environment.${tool.environmentType}`)}</Badge>
        {activeInstallation ? <Badge tone="muted">{t(`cli.source.${activeInstallation.source}`)}</Badge> : null}
        {tool.installations.length > 1 ? (
          <Badge tone="warning">{t("cli.installationsCount", { count: tool.installations.length })}</Badge>
        ) : null}
      </div>

      <dl className="mt-4 grid gap-3 text-sm sm:grid-cols-2">
        <div>
          <dt className="text-xs text-muted-foreground">{t("cli.currentVersion")}</dt>
          <dd className="mt-1 font-mono font-medium">{tool.currentVersion ?? t("cli.versionUnknown")}</dd>
        </div>
        <div>
          <dt className="text-xs text-muted-foreground">{t("cli.latestVersion")}</dt>
          <dd className="mt-1 font-mono font-medium">{tool.latestVersion ?? t("cli.versionUnknown")}</dd>
        </div>
        <div className="sm:col-span-2">
          <dt className="text-xs text-muted-foreground">{t("cli.activePath")}</dt>
          <dd className="mt-1 break-all font-mono text-xs">{tool.activeInstallationPath ?? t("cli.notAvailable")}</dd>
        </div>
      </dl>

      {guidanceText ? (
        <div className="mt-4 flex gap-2 rounded-md border p-3 text-xs ucd-status-warning">
          <AlertTriangle className="mt-0.5 h-4 w-4 shrink-0" aria-hidden="true" />
          <span>{guidanceText}</span>
        </div>
      ) : null}
      {tool.lastError ? <div className="mt-3 rounded-md border p-3 text-xs ucd-status-warning">{tool.lastError}</div> : null}

      <div className="mt-4 flex flex-wrap items-center gap-2">
        {tool.lifecycleEligibility === "npm" ? (
          <select
            aria-label={t("cli.targetVersion", { name: tool.displayName })}
            className="ucd-input h-9 min-w-36 flex-1 rounded px-3 text-sm outline-none focus-visible:ring-2 focus-visible:ring-ring"
            disabled={props.packageBusy || operationRunning || options.length === 0}
            value={props.selectedVersion}
            onChange={(event) => props.onSelectedVersionChange(event.target.value)}
          >
            {options.length === 0 ? <option value="">{t("cli.noVersions")}</option> : null}
            {options.map((version) => <option key={version} value={version}>{version}</option>)}
          </select>
        ) : null}
        {canMutate ? (
          <Button
            disabled={props.packageBusy || operationRunning || !canRunPackageAction}
            title={showsManualUpgrade && guidanceText ? guidanceText : undefined}
            variant={showsManualUpgrade ? "outline" : "default"}
            onClick={props.onRunAction}
          >
            {props.action === "upgrade" ? <ArrowUpCircle aria-hidden="true" /> : <Download aria-hidden="true" />}
            {t(`cli.action.${props.action}`)}
          </Button>
        ) : showsDisabledUpgrade ? (
          <Button disabled title={guidanceText ?? t("cli.action.unavailable")} variant="outline">
            <ArrowUpCircle aria-hidden="true" />
            {t("cli.action.upgrade")}
          </Button>
        ) : props.action === "current" ? (
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
        {props.diagnosticsExpanded ? <div className="mt-3"><CliInstallationList installations={tool.installations} /></div> : null}

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
