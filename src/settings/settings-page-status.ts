import type { SettingsPageStatus, SettingsPageStatusKind } from "./settings-page-types";

/**
 * Highest to lowest: something broken outranks something merely unsaved, which outranks a purely
 * informational heads-up. Shared so a page with more than one true condition at once, and two
 * different pages each with their own single condition, pick the same relative winner a user
 * would expect -- not whichever order that page's own `if` chain happened to check first.
 */
const STATUS_PRIORITY: readonly SettingsPageStatusKind[] = [
  "error",
  "dependency-unavailable",
  "unsaved",
  "restart-required",
  "update-available",
];

/** The one status a nav entry may show (spec.md "Show page status") out of every condition that
 *  is currently true for a page. `null`/`undefined` candidates are conditions that are not
 *  currently true -- pass them through rather than filtering upstream, so a call site can list
 *  every condition it knows how to check regardless of which ones happen to apply right now. */
export function pickPageStatus(
  candidates: readonly (SettingsPageStatus | null | undefined)[],
): SettingsPageStatus | null {
  const present = candidates.filter((candidate): candidate is SettingsPageStatus => Boolean(candidate));
  for (const kind of STATUS_PRIORITY) {
    const match = present.find((candidate) => candidate.kind === kind);
    if (match) return match;
  }
  return null;
}
