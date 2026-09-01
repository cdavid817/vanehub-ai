import { useEffect } from "react";
import { pickPageStatus } from "../settings-page-status";
import type { SettingsPageStatus } from "../settings-page-types";

/**
 * Task 12.16: cross-agent totals, not the active-agent-only ones `onDraftStateChange` uses --
 * this page's nav entry should flag *anything* unsaved here, matching the header's own
 * cross-agent dirty badge, and it keeps reporting while backgrounded (`keepAlive: "draft-only"`,
 * task 12.17), unlike that guard, which only exists to protect the one departure lifecycle can't.
 */
export function useCliParametersPageStatus({
  errorMessage,
  totalDirtyCount,
  onStatusChange,
}: {
  errorMessage: string | null;
  totalDirtyCount: number;
  onStatusChange?: (status: SettingsPageStatus | null) => void;
}) {
  useEffect(() => {
    onStatusChange?.(pickPageStatus([
      errorMessage ? { kind: "error", labelKey: "cliParameters.error.status" } : null,
      totalDirtyCount > 0
        ? { kind: "unsaved", labelKey: "cliParameters.badge.dirty", labelParams: { count: totalDirtyCount } }
        : null,
    ]));
    return () => onStatusChange?.(null);
  }, [errorMessage, onStatusChange, totalDirtyCount]);
}
