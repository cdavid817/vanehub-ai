import { Box, CheckCircle2, Download, Play, Square, Trash2 } from "lucide-react";
import { useTranslation } from "react-i18next";
import { ActionMenu, type ActionMenuItem } from "../../../ui/actions/ActionMenu";
import { MutationStatus } from "../../../ui/async/MutationStatus";
import type { MutationState } from "../../../ui/async/mutation-state";
import { CopyDiagnosticsButton } from "../../../ui/diagnostics/CopyDiagnosticsButton";
import { StatusBadge, type StatusTone } from "../../../ui/status/StatusBadge";
import type {
  ExtensionFrameworkDefinition,
  ExtensionFrameworkId,
  ExtensionFrameworkStatus,
} from "../../../types/extension";
import type { OperationTask } from "../../../types/operation";
import { TagList } from "../page-parts";
import { buildExtensionDiagnosticFields } from "./extension-diagnostic-summary";
import { statusKey } from "./extension-status";

const statusTone: Record<ExtensionFrameworkStatus["status"], StatusTone> = {
  "not-installed": "neutral",
  installing: "warning",
  installed: "neutral",
  starting: "warning",
  running: "success",
  stopping: "warning",
  uninstalling: "warning",
  error: "danger",
  unsupported: "neutral",
};

export function ExtensionFrameworkCard({
  activeOperation,
  definition,
  mutationState,
  nativeAvailable,
  onOpenPreview,
  onRunAction,
  status,
}: {
  activeOperation: OperationTask | undefined;
  definition: ExtensionFrameworkDefinition;
  mutationState: MutationState | undefined;
  nativeAvailable: boolean;
  onOpenPreview: (frameworkId: ExtensionFrameworkId) => void;
  onRunAction: (action: string, frameworkId: ExtensionFrameworkId) => void;
  status: ExtensionFrameworkStatus;
}) {
  const { t } = useTranslation();
  const busy = mutationState?.pending === true;

  // Task 12.18: the page previously rendered up to six conditionally-visible buttons (Requirements
  // is unconditional; Install/Start/Stop/Self-test/Enable-Disable/Uninstall each gate on the
  // framework's own installed/running state, exactly as before) -- collapsed into one ActionMenu
  // per card instead of a growing button row.
  const items: ActionMenuItem[] = [
    {
      icon: Box,
      id: "requirements",
      label: t("extensions.action.requirements"),
      onSelect: () => onOpenPreview(definition.id),
    },
  ];
  if (!status.installed) {
    items.push({
      disabled: !nativeAvailable || busy,
      icon: Download,
      id: "install",
      label: t("extensions.action.install"),
      onSelect: () => onOpenPreview(definition.id),
    });
  }
  if (status.installed && !status.running) {
    items.push({
      disabled: !nativeAvailable || busy,
      icon: Play,
      id: "start",
      label: t("extensions.action.start"),
      onSelect: () => onRunAction("start", definition.id),
    });
  }
  if (status.running) {
    items.push({
      disabled: !nativeAvailable || busy,
      icon: Square,
      id: "stop",
      label: t("extensions.action.stop"),
      onSelect: () => onRunAction("stop", definition.id),
    });
  }
  if (status.installed) {
    items.push(
      {
        disabled: !nativeAvailable || busy,
        icon: CheckCircle2,
        id: "self-test",
        label: t("extensions.action.selfTest"),
        onSelect: () => onRunAction("self-test", definition.id),
      },
      {
        disabled: !nativeAvailable || busy,
        id: status.enabled ? "disable" : "enable",
        label: status.enabled ? t("extensions.action.disable") : t("extensions.action.enable"),
        onSelect: () => onRunAction(status.enabled ? "disable" : "enable", definition.id),
      },
      {
        confirmation: { title: t("extensions.confirm.uninstall") },
        disabled: !nativeAvailable || busy || status.running,
        icon: Trash2,
        id: "uninstall",
        label: t("extensions.action.uninstall"),
        onSelect: () => onRunAction("uninstall", definition.id),
        tone: "destructive",
      },
    );
  }

  return (
    <article className="ucd-panel ucd-interactive rounded-lg p-4" data-testid={`extension-card-${definition.id}`}>
      <div className="flex flex-wrap items-start justify-between gap-3">
        <div className="min-w-0">
          <div className="text-xs font-semibold uppercase tracking-[0.14em] text-primary">
            {t(`extensions.capability.${definition.capabilityId}`)}
          </div>
          <h3 className="mt-1 text-base font-semibold">{t(definition.nameKey)}</h3>
          <p className="mt-1 text-sm leading-6 text-muted-foreground">{t(definition.descriptionKey)}</p>
        </div>
        <div className="flex shrink-0 items-center gap-1">
          <StatusBadge label={t(statusKey(status.status))} tone={statusTone[status.status]} />
          <ActionMenu items={items} triggerLabel={t("extensions.rowActions", { name: t(definition.nameKey) })} />
        </div>
      </div>
      <div className="mt-3 grid gap-3 text-xs text-muted-foreground md:grid-cols-3">
        <div>
          <span className="block">{t("extensions.runtime")}</span>
          <strong className="text-foreground">{definition.requirement.runtime}</strong>
        </div>
        <div>
          <span className="block">{t("extensions.port")}</span>
          <strong className="text-foreground">{status.port}</strong>
        </div>
        <div>
          <span className="block">{t("extensions.disk")}</span>
          <strong className="text-foreground">~{definition.requirement.estimatedDiskMb} MB</strong>
        </div>
      </div>
      <div className="mt-3">
        <TagList tags={definition.requirement.packages} />
      </div>
      {status.lastError ? (
        <div className="mt-3 rounded border p-2 text-xs ucd-status-warning">{t(status.lastError)}</div>
      ) : null}
      <MutationStatus className="mt-3" state={mutationState} />
      {activeOperation ? (
        <div className="mt-3 rounded-md border border-border bg-[hsl(var(--panel-muted))] p-3 text-xs">
          <div className="font-medium">{t("extensions.logs.title")}</div>
          <div className="mt-2 grid gap-1 font-mono text-muted-foreground">
            {activeOperation.logs.map((log) => (
              <div key={`${log.timestamp}-${log.line}`}>{log.line}</div>
            ))}
          </div>
        </div>
      ) : null}
      <div className="mt-3 flex justify-end">
        <CopyDiagnosticsButton fields={buildExtensionDiagnosticFields(definition, status, t)} />
      </div>
    </article>
  );
}
