import { useTranslation } from "react-i18next";
import type { SessionTabId } from "../session-workspace/session-tab-bar";
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
/**
 * Which tab owns each row.
 *
 * Runtime goes to the run list rather than to the records: the row is about a run's state, and the
 * run list is where a reader compares it with the others. Verification goes to the report, which
 * is the surface that aggregates outcomes; diagnostics to the logs that produced the errors.
 *
 * Usage is deliberately absent. It is a pane in this panel, not a workspace tab, and routing it
 * through the same callback would make one destination mean two different kinds of place.
 */
const ROW_TABS = {
  changes: "changes",
  diagnostics: "logs",
  runtime: "traces",
  shells: "shell",
  verification: "report",
  workspace: "files",
} satisfies Record<string, SessionTabId>;

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
export function SessionEvidenceSummary({
  onNavigateToTab,
  onShowUsage,
  sessionId,
}: {
  /** Absent where nothing owns the workspace tabs, in which case the rows are plain text. */
  onNavigateToTab?: (tab: SessionTabId) => void;
  onShowUsage?: () => void;
  sessionId: string | null;
}) {
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
    <ul className="ucd-muted-panel mt-2 grid gap-1 rounded-lg p-3 text-xs">
      <Row
        label={t("layout.info.evidence.runtime")}
        onOpen={onNavigateToTab && (() => onNavigateToTab(ROW_TABS.runtime))}
        value={runtimeLabel(summary, t)}
      />
      <Row
        label={t("layout.info.evidence.workspace")}
        onOpen={onNavigateToTab && (() => onNavigateToTab(ROW_TABS.workspace))}
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
        onOpen={onNavigateToTab && (() => onNavigateToTab(ROW_TABS.shells))}
        value={t("layout.info.evidence.shellsValue", { count: summary.shells.live })}
      />
      <Row
        label={t("layout.info.evidence.changes")}
        onOpen={onNavigateToTab && (() => onNavigateToTab(ROW_TABS.changes))}
        value={t("layout.info.evidence.changesValue", {
          count: summary.changes.changedFiles,
          unviewed: summary.changes.unviewedFiles,
        })}
      />
      <Row
        label={t("layout.info.evidence.verification")}
        onOpen={onNavigateToTab && (() => onNavigateToTab(ROW_TABS.verification))}
        value={t("layout.info.evidence.verificationValue", {
          failed: summary.verification.failed,
          passed: summary.verification.passed,
        })}
      />
      <Row
        label={t("layout.info.evidence.diagnostics")}
        onOpen={onNavigateToTab && (() => onNavigateToTab(ROW_TABS.diagnostics))}
        value={t("layout.info.evidence.diagnosticsValue", {
          count: summary.logs.newErrors,
          running: summary.executionRecords.running,
        })}
      />
      <Row
        label={t("layout.info.evidence.usage")}
        onOpen={onShowUsage}
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
    </ul>
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

/**
 * One row, navigable when there is somewhere for it to go.
 *
 * Plain text otherwise, rather than a disabled control: a row nobody can follow is not a broken
 * action, it is a fact with no destination, and rendering it as a dead button says the opposite.
 *
 * The whole row is the target rather than the value alone. A reader looking for "the changes" aims
 * at the line, and a hit area the width of "8 files" is a hit area most pointers miss.
 */
function Row({ label, onOpen, value }: { label: string; onOpen?: () => void; value: string }) {
  const content = (
    <>
      <span className="w-24 shrink-0 text-muted-foreground">{label}</span>
      <span className="min-w-0 flex-1 truncate">{value}</span>
    </>
  );
  return (
    <li className="min-w-0">
      {onOpen ? (
        // The whole row is the target rather than the value alone. A reader looking for "the
        // changes" aims at the line, and a hit area the width of "8 files" is one most pointers
        // miss. The label and the value together are the accessible name, so what is announced is
        // the same sentence that is on screen.
        <button
          // Named explicitly rather than left to the two spans. The gap between them is flex
          // spacing, not a text node, so the computed name would run the label into the value —
          // "Changes8 files" is what a screen reader would say, and it is not what is on screen.
          aria-label={`${label} ${value}`}
          className="flex w-full min-w-0 items-baseline gap-2 rounded px-1 text-left hover:bg-muted focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring"
          onClick={onOpen}
          type="button"
        >
          {content}
        </button>
      ) : (
        // Plain text rather than a disabled control: a row nobody can follow is not a broken
        // action, it is a fact with no destination, and a dead button says the opposite.
        <span className="flex min-w-0 items-baseline gap-2 px-1">{content}</span>
      )}
    </li>
  );
}
