import { useTranslation } from "react-i18next";
import { formatAppDateTime } from "../../../i18n/format";
import { normalizeDisplayPath } from "../../../lib/session-path";
import { CopyDiagnosticsButton } from "../../../ui/diagnostics/CopyDiagnosticsButton";
import type { CliEnvironmentSnapshot } from "../../../types/cli-environment-snapshot";
import { buildCliDiagnosticFields } from "./cli-diagnostic-summary";
import {
  pathSelectedInstallation,
  recommendedInstallation,
} from "./cli-management-presenters";

/** Timestamps follow the active locale rather than a fixed pattern. */
const DATE_TIME: Intl.DateTimeFormatOptions = { dateStyle: "medium", timeStyle: "short" };

/** Every axis on one screen, each read straight off the snapshot. */
const AXES: Array<[label: string, field: keyof CliEnvironmentSnapshot, prefix: string]> = [
  ["cli.axis.overall", "overallState", "cli.overallState"],
  ["cli.axis.discovery", "discovery", "cli.discovery"],
  ["cli.axis.executable", "executable", "cli.executable"],
  ["cli.axis.authentication", "authentication", "cli.authentication"],
  ["cli.axis.readiness", "readiness", "cli.readiness"],
  ["cli.axis.compatibility", "compatibility", "cli.compatibility"],
  ["cli.axis.update", "update", "cli.update"],
  ["cli.axis.freshness", "freshness", "cli.freshness"],
];

function InstallationLine({ labelKey, path, version }: {
  labelKey: string;
  path: string | undefined;
  version: string | null | undefined;
}) {
  const { t } = useTranslation();
  return (
    <div className="min-w-0">
      <dt className="text-xs text-muted-foreground">{t(labelKey)}</dt>
      <dd className="mt-0.5 break-all font-mono text-xs" title={path}>
        {path ? `${version ?? t("cli.versionUnknown")} · ${normalizeDisplayPath(path)}` : t("cli.notAvailable")}
      </dd>
    </div>
  );
}

export function CliOverviewTab({ snapshot }: { snapshot: CliEnvironmentSnapshot }) {
  const { t, i18n } = useTranslation();
  const pathSelected = pathSelectedInstallation(snapshot);
  const recommended = recommendedInstallation(snapshot);
  const mutation = snapshot.lastMutation;

  return (
    <div className="space-y-4 text-sm">
      <dl className="grid gap-3 sm:grid-cols-2">
        {AXES.map(([label, field, prefix]) => (
          <div key={field}>
            <dt className="text-xs text-muted-foreground">{t(label)}</dt>
            <dd className="mt-0.5 font-medium">{t(`${prefix}.${String(snapshot[field])}`)}</dd>
          </div>
        ))}
      </dl>

      <dl className="grid gap-3 sm:grid-cols-2">
        <InstallationLine
          labelKey="cli.pathSelected"
          path={pathSelected?.executablePath}
          version={pathSelected?.reportedVersion}
        />
        <InstallationLine
          labelKey="cli.recommended"
          path={recommended?.executablePath}
          version={recommended?.reportedVersion}
        />
      </dl>

      <dl className="grid gap-3 sm:grid-cols-2">
        <div>
          <dt className="text-xs text-muted-foreground">{t("cli.lastChecked")}</dt>
          <dd className="mt-0.5">
            {snapshot.checkedAt
              ? formatAppDateTime(snapshot.checkedAt, i18n.language, DATE_TIME)
              : t("cli.neverChecked")}
          </dd>
        </div>
        <div>
          <dt className="text-xs text-muted-foreground">{t("cli.lastMutation")}</dt>
          <dd className="mt-0.5">
            {mutation
              ? `${t(`cli.outcome.${mutation.outcome}`)} · ${t(`cli.source.${mutation.sourceId}`)} · ${formatAppDateTime(mutation.completedAt, i18n.language, DATE_TIME)}`
              : t("cli.notAvailable")}
          </dd>
        </div>
      </dl>

      <div className="flex justify-end">
        <CopyDiagnosticsButton fields={buildCliDiagnosticFields(snapshot, t)} />
      </div>
    </div>
  );
}
