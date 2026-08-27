import { useTranslation } from "react-i18next";
import { evidenceSessionIdSchema } from "../contracts/session-workspace-evidence-ids";
import { useWorkspaceCapabilities } from "../session-workspace/workspace-capability-notice";
import { useWorkspaceEvidenceSummary } from "../session-workspace/use-workspace-evidence-summary";
import type { WorkspaceEvidenceSummary } from "../types/session-workspace-evidence";

/**
 * What this session has actually done, in seven lines.
 *
 * Every number comes from the one summary read the workspace already makes, so the panel agrees
 * with the tab badges rather than measuring the same facts a second time at a different instant.
 * The two are the same query key; adding a second source here would produce a Basic Info that
 * quietly disagrees with the tab bar and no way to tell which is right.
 *
 * A failed read is not an empty session. When the summary is unavailable every row says so instead
 * of rendering zeroes, because "nothing has happened" and "nothing could be read" look identical
 * as numbers and lead a reader to opposite conclusions.
 */
export function SessionEvidenceSummary({ sessionId }: { sessionId: string | null }) {
  const { t } = useTranslation();
  const parsed = sessionId === null ? null : evidenceSessionIdSchema.safeParse(sessionId);
  const evidenceSessionId = parsed?.success ? parsed.data : null;
  const { state, summary } = useWorkspaceEvidenceSummary(evidenceSessionId);
  // The same key the Files, Documents, and Changes tabs read, so this costs a request only when
  // none of them has already asked.
  const { capabilities } = useWorkspaceCapabilities(sessionId);

  if (sessionId === null) return null;
  if (state === "unavailable" || !summary) {
    return (
      <p className="ucd-muted-panel rounded-lg p-3 text-xs text-muted-foreground" role="status">
        {t("layout.info.evidence.unavailable")}
      </p>
    );
  }

  return (
    <dl className="ucd-muted-panel mt-2 grid gap-1 rounded-lg p-3 text-xs">
      <Row label={t("layout.info.evidence.runtime")} value={runtimeLabel(summary, t)} />
      <Row
        label={t("layout.info.evidence.workspace")}
        value={[
          capabilities ? t(`layout.info.evidence.provider.${capabilities.provider}`) : null,
          capabilities?.gitStatus.available ? t("layout.info.evidence.git") : null,
          // Derived from the change count rather than asked for separately: a workspace with
          // changed files is a dirty one, and a second read could disagree with the row below it.
          summary.changes.changedFiles > 0 ? t("layout.info.evidence.dirty") : null,
        ]
          .filter(Boolean)
          .join(" · ")}
      />
      <Row
        label={t("layout.info.evidence.shells")}
        value={t("layout.info.evidence.shellsValue", { count: summary.shells.live })}
      />
      <Row
        label={t("layout.info.evidence.changes")}
        value={t("layout.info.evidence.changesValue", {
          count: summary.changes.changedFiles,
          unviewed: summary.changes.unviewedFiles,
        })}
      />
      <Row
        label={t("layout.info.evidence.verification")}
        value={t("layout.info.evidence.verificationValue", {
          failed: summary.verification.failed,
          passed: summary.verification.passed,
        })}
      />
      <Row
        label={t("layout.info.evidence.diagnostics")}
        value={t("layout.info.evidence.diagnosticsValue", {
          count: summary.logs.newErrors,
          running: summary.executionRecords.running,
        })}
      />
      <Row
        label={t("layout.info.evidence.usage")}
        value={
          // Absent, not zero. A provider that reported no total and a session that spent nothing
          // are different facts, and the second is the one a reader would act on.
          summary.usage.reportedTokens === undefined
            ? t(`layout.info.evidence.usageCoverage.${summary.usage.coverage}`)
            : t("layout.info.evidence.usageValue", {
                count: summary.usage.reportedTokens,
                coverage: t(`layout.info.evidence.usageCoverage.${summary.usage.coverage}`),
              })
        }
      />
    </dl>
  );
}

function runtimeLabel(
  summary: WorkspaceEvidenceSummary,
  t: (key: string, options?: Record<string, unknown>) => string,
): string {
  const status = t(`layout.info.evidence.runState.${summary.runState.status}`);
  // The moment it started, not an elapsed time computed here. A duration rendered from this side
  // would tick against a clock the backend does not share, and would keep ticking for a run that
  // ended while the panel was hidden.
  return summary.runState.startedAt
    ? `${status} · ${t("layout.info.evidence.since", { at: summary.runState.startedAt })}`
    : status;
}

function Row({ label, value }: { label: string; value: string }) {
  return (
    <div className="flex min-w-0 items-baseline gap-2">
      <dt className="w-24 shrink-0 text-muted-foreground">{label}</dt>
      <dd className="min-w-0 flex-1 truncate">{value}</dd>
    </div>
  );
}
