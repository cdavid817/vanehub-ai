import type { TFunction } from "i18next";
import type { DiagnosticField } from "../../../ui/diagnostics/diagnostic-field";
import { normalizeDisplayPath } from "../../../lib/session-path";
import type { CliEnvironmentSnapshot } from "../../../types/cli-environment-snapshot";
import { pathSelectedInstallation, recommendedInstallation } from "./cli-management-presenters";

/**
 * spec.md "Copyable safe settings diagnostics" for one CLI tool's snapshot. `CliEnvironmentSnapshot`
 * has no credential-shaped field anywhere (task 12.19's own audit confirmed this before writing
 * this file) -- every value here is already either a version string, a backend-pinned enum value,
 * a stable id, a path already shown on-screen (`normalizeDisplayPath`, matching this page's own
 * `CliOverviewTab` display -- not a new redaction judgment call), or a raw ISO timestamp (kept
 * unformatted here on purpose: a pasted diagnostic is more useful precise and timezone-unambiguous
 * than locale-pretty).
 */
export function buildCliDiagnosticFields(snapshot: CliEnvironmentSnapshot, t: TFunction): DiagnosticField[] {
  const recommended = recommendedInstallation(snapshot);
  const pathSelected = pathSelectedInstallation(snapshot);
  const installation = recommended ?? pathSelected;
  const recommendedSource = snapshot.sources.find((source) => source.sourceId === installation?.sourceId);
  const conflictCodes = snapshot.conflicts.map((conflict) => conflict.reasonCode);
  const actionCodes = snapshot.allowedActions
    .map((action) => action.reasonCode)
    .filter((code): code is string => code !== null);

  return [
    { label: t("cli.diagnostics.field.version"), value: installation?.reportedVersion ?? null },
    { label: t("cli.axis.overall"), value: snapshot.overallState },
    { label: t("cli.axis.discovery"), value: snapshot.discovery },
    { label: t("cli.axis.executable"), value: snapshot.executable },
    { label: t("cli.axis.authentication"), value: snapshot.authentication },
    { label: t("cli.axis.readiness"), value: snapshot.readiness },
    { label: t("cli.axis.compatibility"), value: snapshot.compatibility },
    { label: t("cli.axis.update"), value: snapshot.update },
    { label: t("cli.axis.freshness"), value: snapshot.freshness },
    { label: t("cli.diagnostics.field.agentId"), value: snapshot.agentId },
    { label: t("cli.diagnostics.field.installationId"), value: installation?.id ?? null },
    { label: t("cli.diagnostics.field.sourceId"), value: installation?.sourceId ?? null },
    { label: t("cli.diagnostics.field.lastOperationId"), value: snapshot.lastOperationId },
    { label: t("cli.diagnostics.field.executablePath"), value: installation ? normalizeDisplayPath(installation.executablePath) : null },
    { label: t("cli.lastChecked"), value: snapshot.checkedAt },
    { label: t("cli.diagnostics.field.lastMutationAt"), value: snapshot.lastMutation?.completedAt ?? null },
    { label: t("cli.diagnostics.field.conflictCodes"), value: conflictCodes.length ? conflictCodes.join(", ") : null },
    { label: t("cli.diagnostics.field.actionReasonCodes"), value: actionCodes.length ? actionCodes.join(", ") : null },
    { label: t("cli.diagnostics.field.sourceGuidanceCode"), value: recommendedSource?.guidanceCode ?? null },
  ];
}
