import type { WorkbenchLocation } from "../main-layout/workbench-route";
import type { SettingsPageId } from "../settings/settings-pages";

/**
 * design.md Decision 4. `route` here is typed `WorkbenchLocation`, not the `WorkbenchRoute` name
 * Decision 4's own snippet uses — `WorkbenchRoute` never appears anywhere else in design.md or in
 * the codebase; `WorkbenchLocation` (workbench-route.ts, built in §4) is the actual, tested,
 * round-tripping type for "a destination plus its addressable state," which is exactly what a
 * search result needs to link to. Reconciling the name here rather than introducing a second,
 * parallel type that would need to stay in sync with it by hand.
 */
export type WorkbenchSearchScope = "session" | "project" | "run" | "goal" | "work-item" | "evaluation";

/**
 * Deliberately small: this redesign's "统一...状态语义" (Decision 4 area, design.md:23) ambition is a
 * separate, cross-app initiative with no implementation anywhere yet (confirmed by search) — this
 * is only the handful of visual categories a search result badge actually needs, not an attempt to
 * unify every domain's own status vocabulary. Domain providers map their own richer states down to
 * one of these when building a `WorkbenchSearchResult`.
 */
export type SemanticStatus = "neutral" | "active" | "attention" | "success" | "error";

export interface WorkbenchSearchRequest {
  query: string;
  scopes: WorkbenchSearchScope[];
  limit: number;
  cursor?: string;
  routeContext?: WorkbenchLocation;
  signal: AbortSignal;
}

/** Mirrors `MissionControlPage`'s existing `{items, nextCursor}` shape (types/mission-control.ts)
 *  rather than inventing a differently-named pagination envelope. */
export interface WorkbenchSearchPage {
  items: WorkbenchSearchResult[];
  nextCursor: string | null;
}

/**
 * Privacy (design.md Decision 4): only local safe summaries. Never a prompt, response, tool input,
 * credential, unredacted path, or raw error — every provider's own file states which of its
 * source fields were deliberately excluded, not just which were included.
 */
export interface WorkbenchSearchResult {
  key: string;
  kind: WorkbenchSearchScope;
  title: string;
  subtitle?: string;
  status?: SemanticStatus;
  route: WorkbenchLocation;
  updatedAt?: string;
  keywords?: string[];
}

export interface WorkbenchSearchProvider {
  id: string;
  supports(scope: WorkbenchSearchScope): boolean;
  search(request: WorkbenchSearchRequest): Promise<WorkbenchSearchPage>;
}

/**
 * What a `WorkbenchCommand` needs to decide availability and act. `navigate`/`onOpenSettings`
 * mirror `MainLayout`'s own prop names exactly (main-layout.tsx) rather than renaming them, since
 * every command's `run()` is ultimately just calling one of these through.
 */
export interface WorkbenchCommandContext {
  location: WorkbenchLocation;
  navigate: (next: WorkbenchLocation, options?: { replace?: boolean }) => void;
  onOpenSettings: (pageId?: SettingsPageId) => void;
  onNewSession: () => void;
  onToggleNavigation: () => void;
  onToggleInspector: () => void;
  onToggleFocusMode: () => void;
}

export interface WorkbenchCommand {
  id: string;
  labelKey: string;
  keywords: string[];
  isAvailable(context: WorkbenchCommandContext): boolean;
  run(context: WorkbenchCommandContext): Promise<void> | void;
}
