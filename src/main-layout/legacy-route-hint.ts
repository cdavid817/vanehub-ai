/**
 * 4.14: the one-time "moved" hint shown the first time a legacy route (workbench-route.ts's
 * `legacyWorkbenchRedirectPath`) redirects someone. Stores only a versioned dismissal flag, per
 * the task's own constraint — not which specific old link was followed, not a count, nothing else
 * that would need its own migration or cleanup later.
 */
const DISMISSED_KEY = "vanehub.workbench.legacy-route-hint-dismissed.v1";

/** Fails closed to "already seen": a storage-less environment should not nag on every visit. */
export function hasSeenLegacyRouteHint(): boolean {
  if (typeof localStorage === "undefined") return true;
  return localStorage.getItem(DISMISSED_KEY) === "true";
}

export function markLegacyRouteHintSeen(): void {
  if (typeof localStorage === "undefined") return;
  localStorage.setItem(DISMISSED_KEY, "true");
}
