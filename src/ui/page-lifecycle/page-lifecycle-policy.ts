/**
 * design.md Decision 6. A page's declared contract for what happens to it while it is not the
 * one the reader is looking at — separate from whether a `LazyFeature` chooses to mount it at
 * all, and separate from whatever a service keeps running on its own behalf regardless of any UI.
 */
export interface PageLifecyclePolicy {
  /**
   * `"never"`: unmount when not the active page (the default — Decision 6: "默认 keepAlive:
   * never"). `"draft-only"`: keep mounted only because in-progress form state would otherwise be
   * lost and is not reasonably serializable elsewhere — needs a documented reason at its call
   * site. `"always"`: keep mounted for a reason beyond form state (e.g. a live connection whose
   * reconnect cost matters) — also needs a documented reason; Decision 6 requires this be
   * explained in comments and tests, not just declared.
   */
  keepAlive: "never" | "draft-only" | "always";
  /** Whether a `keepAlive` page must pause its own polling/timers/subscriptions while hidden. */
  suspendWhenHidden: boolean;
  /** Whether becoming active again triggers a bounded reconciliation fetch, not a full reload. */
  refreshOnFocus: boolean;
  /** What a hidden `keepAlive` page may still update in the background. `"none"` for `never`
   *  pages, which have nothing left mounted to update. */
  backgroundUpdates: "none" | "terminal-only" | "all";
}

/** The declared default for any page without a documented reason to differ. */
export const DEFAULT_PAGE_LIFECYCLE_POLICY: PageLifecyclePolicy = {
  keepAlive: "never",
  suspendWhenHidden: true,
  refreshOnFocus: false,
  backgroundUpdates: "none",
};

/**
 * The one decision every `keepAlive`-aware page list (Settings' page grid today) needs, pulled out
 * so it is unit-testable without mounting a real page tree: render the active page always, and a
 * previously-visited inactive one only if its policy says to keep it alive.
 */
export function shouldRenderPage(policy: PageLifecyclePolicy, isActivePage: boolean, everVisited: boolean): boolean {
  if (isActivePage) return true;
  return policy.keepAlive !== "never" && everVisited;
}
