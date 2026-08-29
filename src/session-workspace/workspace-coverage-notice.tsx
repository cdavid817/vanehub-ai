import { AlertTriangle } from "lucide-react";
import { useTranslation } from "react-i18next";
import type { WorkspaceInspectionProviderId } from "../types/session-workspace";
import { searchReasonKey } from "./search-coverage";

/**
 * Why a panel is not showing everything.
 *
 * This replaces one shared "results are partial" line that four surfaces used for four different
 * reasons. That line was true everywhere and useful nowhere: a reader who saw it could not tell
 * whether a folder had more entries, a discovery walk had stopped, a status list had been cut, or a
 * diff had hit a byte bound — and those have four different next actions.
 *
 * The provider is part of the message because the remediation differs by machine. "This folder has
 * more entries than one page" is the same fact locally and remotely; "the walk stopped early" on a
 * host across a network is a different problem from the same words about a local disk.
 */
export type WorkspaceCoverageReason =
  /** A directory holds more entries than one page. More are reachable. */
  | "directory-page"
  /** The document discovery walk stopped at its own bound. */
  | "document-walk"
  /** More changed paths exist than the status bound returns. */
  | "git-status-bound"
  /** The diff was cut at its byte bound, so what is shown is a smaller change than the real one. */
  | "git-diff-bound"
  /**
   * The listing itself did not finish, so entries are missing rather than merely further down.
   *
   * Its own reason rather than reusing `directory-page`, because the two suggest opposite actions.
   * "More entries than one page" tells a reader to keep going; a scan that stopped, or a resume
   * point the folder outgrew, tells them the list in front of them is not the folder — and scrolling
   * will not produce the rest.
   */
  | "directory-incomplete";

export function WorkspaceCoverageNotice({
  provider,
  reason,
  reasonCode,
}: {
  /** Absent while capabilities are still loading, in which case the message stays provider-neutral. */
  provider?: WorkspaceInspectionProviderId;
  reason: WorkspaceCoverageReason;
  /**
   * The stable code the listing gave, when it gave one.
   *
   * Worded through the shared reason table so a directory notice and a search notice say the same
   * thing about the same stop, and dropped when this build has no wording for it — a raw
   * `stale_cursor` in front of a reader is worse than the sentence it would have replaced.
   */
  reasonCode?: string | null;
}) {
  const { t } = useTranslation();
  const detail = searchReasonKey(reasonCode ?? undefined);
  return (
    <p
      className="flex items-center gap-1.5 rounded border border-border bg-muted px-2 py-1 text-xs text-muted-foreground"
      role="status"
    >
      <AlertTriangle aria-hidden="true" className="h-3 w-3 shrink-0 text-primary" />
      <span>
        {t(`sessionTabs.coverage.${reason}`)}
        {detail ? ` ${t(detail)}` : null}
        {/* Appended rather than woven in, so a translation does not have to carry a machine name in
            the middle of a sentence — and so a workspace on this machine says nothing extra. */}
        {provider && provider !== "local" ? ` ${t(`sessionTabs.coverage.provider.${provider}`)}` : null}
      </span>
    </p>
  );
}
