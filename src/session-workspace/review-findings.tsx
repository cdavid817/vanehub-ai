import { useTranslation } from "react-i18next";
import type { ReviewFinding } from "../types/code-review";

/**
 * What an automated check said, and how to get to what it was looking at.
 *
 * A finding used to render as one line of text, which left a reviewer with a claim and no way to
 * check it: the run that produced it lives in the execution records, and the code it is about is
 * two clicks away in a file list. Both are reachable from what the finding already carries.
 *
 * The operation id is the only correlation a finding holds, and it is enough — the records tab
 * resolves the run, the trace, and the span from it. Carrying those here as well would be three
 * more identifiers this side would have to keep agreeing with a store it cannot read.
 */
export function ReviewFindings({
  findings,
  onShowCode,
  onShowOperation,
}: {
  findings: readonly ReviewFinding[];
  /** Selects the finding's file inside this panel. Anchored findings only. */
  onShowCode: (path: string) => void;
  /** Absent where nothing owns the evidence scope, in which case the link is not offered. */
  onShowOperation?: (operationId: string) => void;
}) {
  const { t } = useTranslation();
  return (
    <>
      {findings.map((finding) => (
        <div
          className="flex flex-wrap items-center gap-2 rounded border border-border p-2 text-xs"
          key={finding.id}
        >
          <span className="min-w-0 flex-1 break-words">
            {finding.severity}: {finding.title}
          </span>
          {/* Offered only where it leads somewhere. A link to an operation nobody can navigate to
              is a control that reports the application as broken when it is pressed. */}
          {onShowOperation ? (
            <button
              className="rounded border border-border px-2 py-1"
              onClick={() => onShowOperation(finding.operationId)}
              type="button"
            >
              {t("sessionTabs.review.findingRun")}
            </button>
          ) : null}
          {finding.anchor ? (
            <button
              className="rounded border border-border px-2 py-1"
              onClick={() => onShowCode(finding.anchor?.filePath ?? "")}
              type="button"
            >
              {t("sessionTabs.review.findingCode")}
            </button>
          ) : null}
        </div>
      ))}
    </>
  );
}
