import type { LazyFeatureLoader } from "../../components/lazy-feature";
import type { ChatMessage } from "../../types/chat";
import type { LoopInspectionTarget } from "../../types/loop";
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
  /**
   * Absent where nothing owns the workspace tabs, mirroring the old SessionInfoPanel's own
   * optional prop. Typed as a plain `string`, not `SessionSurfaceId` (ARCH-FE-005: `src/ui/`
   * primitives stay feature-agnostic, no importing a `src/session-workspace/` type) — the caller
   * that implements this callback owns the real, narrower type, the same way it owns validating
   * `requestedSessionSection` below.
   */
  onNavigateToSessionTab?: (tab: string) => void;
  /**
   * Opaque to this layer — meaningful only to whichever provider's own content knows its section
   * ids (currently just Session Overview's `"usage"`/`"im"`). Sourced the same way the
   * pre-migration information panel's own `requestedTab` was: a value that is set but never reset
   * back to `null`, so re-requesting the same section twice in a row is a harmless no-op rather
   * than something that needs a nonce to distinguish from "unchanged" — see `main-layout.tsx`.
   */
  requestedSessionSection?: string | null;
  /**
   * Live turn state a provider cannot derive from `selection` alone: who is currently speaking,
   * and the transcript to derive that from. Only meaningful when the selected session is the one
   * actually streaming right now (a session referenced from elsewhere is neither, and simply
   * receives neither field) — mirrors the pre-migration information panel's own
   * `currentSpeakerSeatId`/`messages` props, sourced from the same `model.turnStatus`/
   * `model.messages` in `main-layout.tsx`.
   */
  currentSpeakerSeatId?: string | null;
  messages?: ChatMessage[];
  /**
   * Cross-navigates to the session backing a loop iteration's worker/verifier evidence -- Loop
   * Center's own pre-existing "inspect this session's logs/changes/files" affordance
   * (`LoopInspectionActions`), threaded through so the `loop-iteration` provider can keep offering
   * it from inside the shared Inspector shell. Unlike `onNavigateToSessionTab` above,
   * `LoopInspectionTarget` is imported directly rather than widened to a plain shape: it lives in
   * `src/types/loop.ts`, not a `src/loop-center/`-owned module, so importing it here does not cross
   * the ARCH-FE-005 feature boundary the way importing a `src/session-workspace/`-owned type would.
   */
  onInspectLoop?: (target: LoopInspectionTarget) => void;
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
export const INSPECTOR_PROVIDERS: { [K in WorkbenchSelectionKind]?: InspectorProvider<K> } = {
  session: {
    kind: "session",
    titleKey: "workbenchUi.inspector.title.session",
    loader: () => import("../../main-layout/session-overview").then((module) => ({ default: module.SessionOverview })),
  },
  "loop-iteration": {
    kind: "loop-iteration",
    titleKey: "workbenchUi.inspector.title.loop-iteration",
    loader: () => import("../../loop-center/loop-iteration-inspector-provider").then((module) => ({ default: module.LoopIterationInspectorProvider })),
  },
};

export function getInspectorProvider<K extends WorkbenchSelectionKind>(kind: K): InspectorProvider<K> | undefined {
  return INSPECTOR_PROVIDERS[kind];
}
