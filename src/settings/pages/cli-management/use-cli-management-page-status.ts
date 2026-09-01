import { useEffect } from "react";
import { pickPageStatus } from "../../settings-page-status";
import type { SettingsPageStatus } from "../../settings-page-types";

/**
 * Task 12.16: extracted the same way `use-cli-parameters-page-status.ts` was -- wiring this
 * inline pushes `cli-management-page.tsx` over this repo's 300-line ESLint limit. `updateCount` is
 * `counts.updates` from `summaryCounts` (`cli-management-presenters.ts`), the same number
 * `CliSummaryBar` already renders live. There is deliberately no `dependency-unavailable` signal
 * here -- this page's own "broken"/"missing" bucket semantics don't map cleanly onto that
 * condition, per the prior audit.
 */
export function useCliManagementPageStatus({
  error,
  updateCount,
  onStatusChange,
}: {
  error: unknown;
  updateCount: number;
  onStatusChange?: (status: SettingsPageStatus | null) => void;
}) {
  useEffect(() => {
    onStatusChange?.(pickPageStatus([
      error ? { kind: "error", labelKey: "cli.status.error" } : null,
      updateCount > 0
        ? { kind: "update-available", labelKey: "cli.status.updateAvailable", labelParams: { count: updateCount } }
        : null,
    ]));
    return () => onStatusChange?.(null);
  }, [error, onStatusChange, updateCount]);
}
