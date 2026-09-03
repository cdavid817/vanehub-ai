import { workspaceViews, type WorkspaceView } from "./workspace-filter";

/**
 * 13.12: "restore list filters and scroll anchor on Back." Mirrors
 * `mission-control-view-state.ts`'s own established shape exactly (same `sessionStorage`
 * rationale: this is the reader's current working filter/scroll position, not a durable
 * preference that should survive days between launches). Two round trips need this, not one --
 * `Projects` itself fully unmounts on a destination switch (lazy-loaded via `LazyFeature`), *and*
 * the compact/narrow layout swaps the visible pane between list and detail in place, which can
 * clamp the shared scroll container's `scrollTop` even though the DOM node itself survives (see
 * `projects.tsx`'s own doc comment on why a plain `useState`/`useRef` is not enough for either
 * case).
 */
const VIEW_KEY = "vanehub.projects.view.v1";
const SCROLL_KEY = "vanehub.projects.scroll.v1";

export function readProjectsView(): WorkspaceView | null {
  if (typeof sessionStorage === "undefined") return null;
  const raw = sessionStorage.getItem(VIEW_KEY);
  return raw && (workspaceViews as string[]).includes(raw) ? (raw as WorkspaceView) : null;
}

export function writeProjectsView(view: WorkspaceView): void {
  if (typeof sessionStorage === "undefined") return;
  sessionStorage.setItem(VIEW_KEY, view);
}

export function readProjectsScrollTop(): number {
  if (typeof sessionStorage === "undefined") return 0;
  const raw = sessionStorage.getItem(SCROLL_KEY);
  const value = raw ? Number(raw) : 0;
  return Number.isFinite(value) && value >= 0 ? value : 0;
}

export function writeProjectsScrollTop(scrollTop: number): void {
  if (typeof sessionStorage === "undefined") return;
  sessionStorage.setItem(SCROLL_KEY, String(scrollTop));
}
