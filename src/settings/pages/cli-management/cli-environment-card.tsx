import { AlertTriangle, ArrowUpCircle, CheckCircle2, Info, RefreshCw } from "lucide-react";
import { useTranslation } from "react-i18next";
import { AgentBrandIcon } from "../../../components/agent-brand-icon";
import { Badge } from "../../../components/ui/badge";
import { Button } from "../../../components/ui/button";
import { getAgentVisualIdentity } from "../../../lib/agent-visual-identity";
import { normalizeDisplayPath } from "../../../lib/session-path";
import type { CliEnvironmentSnapshot } from "../../../types/cli-environment-snapshot";
import type { OperationTask } from "../../../types/operation";
import {
  blockingConflict,
  canRequestChange,
  installedVersion,
  pathDivergesFromRecommended,
  pathSelectedInstallation,
  recommendedInstallation,
  recommendedSourceId,
  sourceSummary,
  targetVersionOptions,
} from "./cli-management-presenters";
import { CliOperationStatus, isOperationRunning } from "./cli-operation-status";
import { CliStatusBadges } from "./cli-status-badges";

interface CliEnvironmentCardProps {
  snapshot: CliEnvironmentSnapshot;
  selectedVersion: string;
  operation?: OperationTask;
  refreshing: boolean;
  /** Per tool, never global: one tool's mutation has no bearing on another tool's buttons. */
  mutating: boolean;
  /** Whether this card's drawer is the one on screen, so the trigger can report its own state. */
  detailsOpen: boolean;
  /** The id the drawer renders under, for `aria-controls` on the trigger. */
  detailsPanelId: string;
  onSelectedVersionChange: (version: string) => void;
  onRefresh: () => void;
  /**
   * The dialogs restore focus to the control that opened them, so both callbacks hand over the
   * element itself. Reading `document.activeElement` instead works in a browser and silently
   * resolves to `<body>` for a click that never moved focus.
   */
  onRequestChange: (targetVersion: string, trigger: HTMLElement) => void;
  onOpenDetails: (trigger: HTMLElement) => void;
  onCancelOperation: () => void;
}

export function CliEnvironmentCard(props: CliEnvironmentCardProps) {
  const { t } = useTranslation();
  const { snapshot, operation } = props;
  const options = targetVersionOptions(snapshot);
  const installed = installedVersion(snapshot);
  const sourceId = recommendedSourceId(snapshot);
  const source = sourceSummary(snapshot, sourceId);
  const identity = getAgentVisualIdentity(snapshot.agentId);
  const pathSelected = pathSelectedInstallation(snapshot);
  const recommended = recommendedInstallation(snapshot);
  const conflict = blockingConflict(snapshot);
  const operationRunning = isOperationRunning(operation);
  // With no stored choice the browser shows the first option, so the button acts on that one.
  const selected = props.selectedVersion || options[0] || null;
  const changeable = canRequestChange(snapshot, selected);

  return (
    <section
      className="ucd-panel ucd-interactive flex flex-col gap-3 rounded-lg p-4"
      data-cli-agent={snapshot.agentId}
    >
      <div className="flex items-start justify-between gap-3">
        <div className="flex min-w-0 items-start gap-3">
          <span className={`flex h-9 w-9 shrink-0 items-center justify-center rounded-xl border ${identity.tone}`}>
            <AgentBrandIcon agentId={snapshot.agentId} className="h-5 w-5" />
          </span>
          <div className="min-w-0">
            <h3 className="truncate font-semibold">{snapshot.displayName}</h3>
            <p className="mt-0.5 truncate text-xs text-muted-foreground">
              {snapshot.provider} · {t(`cli.overallState.${snapshot.overallState}`)}
            </p>
          </div>
        </div>
        <div className="flex shrink-0 items-center gap-1">
          <Button
            aria-controls={props.detailsOpen ? props.detailsPanelId : undefined}
            aria-expanded={props.detailsOpen}
            aria-haspopup="dialog"
            aria-label={t("cli.details.open", { name: snapshot.displayName })}
            size="icon"
            title={t("cli.details.open", { name: snapshot.displayName })}
            variant="ghost"
            onClick={(event) => props.onOpenDetails(event.currentTarget)}
          >
            <Info aria-hidden="true" />
          </Button>
          <Button
            aria-label={t("cli.refreshOne", { name: snapshot.displayName })}
            disabled={props.refreshing}
            size="icon"
            title={t("cli.refreshOne", { name: snapshot.displayName })}
            variant="ghost"
            onClick={props.onRefresh}
          >
            <RefreshCw aria-hidden="true" className={props.refreshing ? "animate-spin" : ""} />
          </Button>
        </div>
      </div>

      <CliStatusBadges snapshot={snapshot} />

      <dl className="grid gap-2 text-sm sm:grid-cols-2">
        <div className="min-w-0">
          <dt className="text-xs text-muted-foreground">{t("cli.pathSelected")}</dt>
          <dd className="mt-0.5 truncate font-mono text-xs" title={pathSelected?.executablePath}>
            {pathSelected
              ? `${pathSelected.reportedVersion ?? t("cli.versionUnknown")} · ${normalizeDisplayPath(pathSelected.executablePath)}`
              : t("cli.notOnPath")}
          </dd>
        </div>
        {/* Only when they differ: repeating the same line twice says nothing. */}
        {pathDivergesFromRecommended(snapshot) && recommended ? (
          <div className="min-w-0">
            <dt className="text-xs text-muted-foreground">{t("cli.recommended")}</dt>
            <dd className="mt-0.5 truncate font-mono text-xs" title={recommended.executablePath}>
              {`${recommended.reportedVersion ?? t("cli.versionUnknown")} · ${normalizeDisplayPath(recommended.executablePath)}`}
            </dd>
          </div>
        ) : null}
      </dl>

      <div className="flex flex-wrap items-center gap-2 text-xs">
        {sourceId ? <Badge tone="muted">{t(`cli.source.${sourceId}`)}</Badge> : null}
        {recommended ? (
          <span className="text-muted-foreground">
            {t(`cli.confidence.${recommended.sourceConfidence}`)}
          </span>
        ) : null}
        {source ? (
          <Badge tone={source.management === "managed" ? "muted" : "warning"}>
            {t(`cli.management.${source.management}`)}
          </Badge>
        ) : null}
      </div>

      {conflict ? (
        <div className="flex gap-2 rounded-md border p-2 text-xs ucd-status-warning">
          <AlertTriangle aria-hidden="true" className="mt-0.5 h-4 w-4 shrink-0" />
          <span>{t(`cli.conflict.${conflict.reasonCode}`)}</span>
        </div>
      ) : null}
      {source?.guidanceCode ? (
        <p className="text-xs text-muted-foreground">{t(source.guidanceCode)}</p>
      ) : null}

      <div className="mt-auto flex flex-wrap items-center gap-2">
        {options.length > 0 ? (
          <select
            aria-label={t("cli.targetVersion", { name: snapshot.displayName })}
            className="ucd-input h-9 min-w-32 flex-1 rounded px-3 text-sm outline-hidden focus-visible:ring-2 focus-visible:ring-ring"
            disabled={props.mutating || operationRunning}
            value={selected ?? ""}
            onChange={(event) => props.onSelectedVersionChange(event.target.value)}
          >
            {options.map((version) => (
              <option key={version} value={version}>{version}</option>
            ))}
          </select>
        ) : null}
        {changeable ? (
          <Button
            disabled={props.mutating || operationRunning}
            onClick={(event) => selected && props.onRequestChange(selected, event.currentTarget)}
          >
            <ArrowUpCircle aria-hidden="true" />
            {t("cli.action.change")}
          </Button>
        ) : selected !== null && selected === installed ? (
          <span className="inline-flex h-9 items-center gap-2 text-xs text-muted-foreground">
            <CheckCircle2 aria-hidden="true" className="h-4 w-4 text-[hsl(var(--success))]" />
            {t("cli.action.current")}
          </span>
        ) : null}
      </div>

      {operation ? (
        <CliOperationStatus operation={operation} onCancel={props.onCancelOperation} />
      ) : null}
    </section>
  );
}
