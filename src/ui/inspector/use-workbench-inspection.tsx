import { useCallback, useEffect, useMemo, useState, type ReactNode } from "react";
import { useTranslation } from "react-i18next";
import { LazyFeature } from "../../components/lazy-feature";
import { isSelectionInScope, type WorkbenchSelection, type WorkbenchSelectionKind, type WorkbenchSelectionScope } from "../../types/workbench-selection";
import type { AsyncViewState } from "../async/async-view-state";
import { getInspectorProvider, type InspectorProviderContext } from "./inspector-provider-registry";
import type { InspectorMode } from "./Inspector";

const TITLE_KEYS: Record<WorkbenchSelectionKind, string> = {
  session: "workbenchUi.inspector.title.session",
  message: "workbenchUi.inspector.title.message",
  tool: "workbenchUi.inspector.title.tool",
  file: "workbenchUi.inspector.title.file",
  change: "workbenchUi.inspector.title.change",
  run: "workbenchUi.inspector.title.run",
  "loop-iteration": "workbenchUi.inspector.title.loop-iteration",
  "evaluation-result": "workbenchUi.inspector.title.evaluation-result",
};

export interface WorkbenchInspection {
  mode: InspectorMode;
  selection: WorkbenchSelection | null;
  title: string;
  /** Ready to hand straight to `Inspector`'s `detail` prop — see the file doc comment. */
  detail: AsyncViewState<ReactNode>;
  /** No-op while `pinned`: Decision 8 — a pinned selection is not replaced by the next click. */
  follow: (selection: WorkbenchSelection) => void;
  /** No-op in `overview` mode — nothing to pin. */
  pin: () => void;
  unpin: () => void;
  returnToOverview: () => void;
}

/**
 * The selection/pin/mode state machine behind design.md Decision 8, independent of how a caller
 * chooses to render it (that stays with `Inspector` + whatever `overview`/`onClose` the caller
 * supplies — this hook has no opinion on either).
 *
 * `detail` resolves in two layers, not one: this hook only ever answers a synchronous question —
 * is there a selection, is it in scope, is a provider registered for its kind — and hands off
 * entirely to the provider component for the real, per-kind async query (task 9.3: "providers
 * SHALL use owning frontend services only", which this hook cannot do on a provider's behalf
 * without importing every owning service itself). A provider failing its own query (permission,
 * version, deleted — task 9.13's non-scope triggers) renders that failure inside the same content
 * area via its own `AsyncBoundary`, one layer below this hook's `detail`; nothing here needs to
 * know which of those applies, only that scope is the one check no provider could make for
 * itself, since a provider is never told what is currently on screen elsewhere.
 */
export function useWorkbenchInspection(scope: WorkbenchSelectionScope, context: InspectorProviderContext = {}): WorkbenchInspection {
  const { t } = useTranslation();
  const [selection, setSelection] = useState<WorkbenchSelection | null>(null);
  const [pinned, setPinned] = useState(false);

  const outOfScope = selection !== null && !isSelectionInScope(selection, scope);

  useEffect(() => {
    // 9.13: a non-pinned selection auto-clears once scope drifts (e.g. the reader switched
    // sessions). A pinned one is left alone so 9.14's "explain what changed" has an identity to
    // explain — `unavailable` rendering for it comes from the `outOfScope` branch below, not from
    // clearing `selection` here.
    if (!pinned && outOfScope) setSelection(null);
  }, [pinned, outOfScope]);

  const follow = useCallback(
    (next: WorkbenchSelection) => {
      if (pinned) return;
      setSelection(next);
    },
    [pinned],
  );

  const pin = useCallback(() => {
    setPinned((current) => (selection ? true : current));
  }, [selection]);

  const unpin = useCallback(() => setPinned(false), []);

  const returnToOverview = useCallback(() => {
    setSelection(null);
    setPinned(false);
  }, []);

  const mode: InspectorMode = selection === null ? "overview" : pinned ? "pinned" : "follow";
  const title = t(selection ? TITLE_KEYS[selection.kind] : "workbenchUi.inspector.overviewTitle");

  const detail = useMemo<AsyncViewState<ReactNode>>(() => {
    if (!selection) return { initialLoading: false, refreshing: false, stale: false };
    if (outOfScope) {
      return {
        initialLoading: false,
        refreshing: false,
        stale: false,
        error: { kind: "unavailable", message: t("workbenchUi.evidence.unavailable"), retryable: false },
      };
    }
    const provider = getInspectorProvider(selection.kind);
    if (!provider) {
      return {
        initialLoading: false,
        refreshing: false,
        stale: false,
        error: { kind: "unavailable", message: t("workbenchUi.evidence.unavailable"), retryable: false },
      };
    }
    return {
      // Keyed by kind, not left to default reconciliation: `LazyFeature` only calls `lazy(loader)`
      // once, in its own `useState` initializer, and a `loader` prop that changes on a later
      // render does not retrigger it — without this key, following a message selection with a
      // file selection would keep rendering the message provider forever. Keyed by kind rather
      // than by full selection identity so that following one message with another *within* the
      // same kind updates via normal props/query-key reactivity instead of a full remount.
      data: <LazyFeature componentProps={{ context, selection }} key={selection.kind} loader={provider.loader} />,
      initialLoading: false,
      refreshing: false,
      stale: false,
    };
  }, [selection, outOfScope, t, context]);

  return { detail, follow, mode, pin, returnToOverview, selection, title, unpin };
}
