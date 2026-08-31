import type { LazyFeatureLoader } from "../../components/lazy-feature";
import type { SessionSurfaceId } from "../../session-workspace/session-surface-registry";
import type { WorkbenchSelection, WorkbenchSelectionKind } from "../../types/workbench-selection";

/**
 * Ambient app-level navigation a provider may need but cannot reach through `selection` or its
 * own owning service — mirrors `SettingsPageContext` (settings-page-types.ts), the established
 * precedent for lazily-loaded, id-addressed content that still needs a few host-supplied
 * callbacks. Everything reachable by a plain route instead goes through `EvidenceLink`, not this
 * bag — `onNavigateToSessionTab` exists here only because session workspace tabs are reducer
 * state on the current route (workspace-evidence-reducer.ts), not a distinct URL a `Link` could
 * target.
 */
export interface InspectorProviderContext {
  /** Absent where nothing owns the workspace tabs, mirroring the old SessionInfoPanel's own optional prop. */
  onNavigateToSessionTab?: (tab: SessionSurfaceId) => void;
}

/** What every Inspector provider component receives — never more than its own kind's selection. */
export interface InspectorProviderProps<K extends WorkbenchSelectionKind = WorkbenchSelectionKind> {
  selection: Extract<WorkbenchSelection, { kind: K }>;
  context: InspectorProviderContext;
}

export interface InspectorProvider<K extends WorkbenchSelectionKind = WorkbenchSelectionKind> {
  kind: K;
  /** i18n key for the Inspector header while this provider's detail is shown. */
  titleKey: string;
  /**
   * design.md Decision 8: "详情 Provider 按 kind lazy load". Mirrors `SettingsPageDefinition`'s
   * `loader` (settings-page-types.ts), not Command Center's eager provider array — Command
   * Center's providers all run on every keystroke of a live search, so eagerly importing them
   * pays off; an Inspector provider runs only while its one kind is selected, so most of the
   * bundle for the other seven kinds a reader never selects should never load at all.
   */
  loader: LazyFeatureLoader<InspectorProviderProps<K>>;
}

/**
 * One entry per selection kind, added as each kind's real detail content is built — task 9.3 is
 * this mechanism, not a mandate that all eight kinds ship detail content in this same change.
 * Mirrors `settings-pages.ts`: a plain, directly-authored list, not a mutable registration API —
 * there is exactly one place a reader checks to see what a kind resolves to.
 *
 * A kind with no entry is not a bug: `useWorkbenchInspection` reports it as the same
 * `unavailable` state as a selection whose object was deleted (task 9.8) — the correct, honest
 * behavior for "nothing to show yet" rather than a placeholder screen.
 */
export const INSPECTOR_PROVIDERS: { [K in WorkbenchSelectionKind]?: InspectorProvider<K> } = {};

export function getInspectorProvider<K extends WorkbenchSelectionKind>(kind: K): InspectorProvider<K> | undefined {
  return INSPECTOR_PROVIDERS[kind];
}
