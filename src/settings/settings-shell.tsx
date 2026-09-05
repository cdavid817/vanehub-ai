import { useCallback, useEffect, useMemo, useRef, useState } from "react";
import { useTranslation } from "react-i18next";
import { useSearchParams } from "react-router";
import { LazyFeature } from "../components/lazy-feature";
import { useDraftNavigationGuard } from "../components/ui/use-draft-navigation-guard";
import { useSettingsAnchorHighlight } from "../hooks/use-settings-anchor-highlight";
import { shouldRenderPage } from "../ui/page-lifecycle/page-lifecycle-policy";
import { SETTINGS_PAGE_LIFECYCLE } from "./settings-page-lifecycle";
import type { SettingsDraftGuard, SettingsPageStatus } from "./settings-page-types";
import { defaultSettingsPageId, getSettingsPage, settingsPages, type SettingsNavigationTarget, type SettingsPageId } from "./settings-pages";
import { buildSettingsSearchIndex, type SettingsSearchResult } from "./settings-search-index";
import { SettingsCompactNav } from "./settings-compact-nav";
import { SettingsSidebar } from "./settings-sidebar";
import { SettingsTopBar } from "./settings-topbar";

/** Static registry data, built once -- `settingsPages` never changes at runtime (task 12.4). */
const settingsSearchIndex = buildSettingsSearchIndex(settingsPages);

export function SettingsShell({
  initialNavigationTarget = null,
  initialPageId = defaultSettingsPageId,
  onOpenSession,
  onReturn,
}: {
  initialNavigationTarget?: SettingsNavigationTarget | null;
  initialPageId?: SettingsPageId;
  onOpenSession?: (sessionId: string) => void;
  onReturn?: () => void;
}) {
  const { t } = useTranslation();
  const [searchParams] = useSearchParams();
  const requestedPage = searchParams.get("section");
  const initialPage = settingsPages.some((page) => page.id === requestedPage) ? requestedPage as SettingsPageId : initialPageId;
  const [activePageId, setActivePageId] = useState<SettingsPageId>(initialPage);
  const [visitedPages, setVisitedPages] = useState<Set<SettingsPageId>>(
    () => new Set([initialPage]),
  );
  const [navigationTarget, setNavigationTarget] = useState<SettingsNavigationTarget | null>(initialNavigationTarget);
  const [searchTerm, setSearchTerm] = useState("");
  const [pendingAnchorId, setPendingAnchorId] = useState<string | null>(null);
  // A ref, not state: the guard is only ever read imperatively, at the moment a navigation is
  // attempted, never rendered -- state here would re-render the shell (and, through fresh
  // `pageProps`, the active page) on every report, and a reporting page's own effect naturally
  // depends on non-primitive values (its mutation/draft-API objects) that are new every render,
  // which is exactly the shape of an infinite report-render-report loop.
  const draftGuardRef = useRef<SettingsDraftGuard | null>(null);
  const handleDraftStateChange = useCallback((guard: SettingsDraftGuard | null) => {
    draftGuardRef.current = guard;
  }, []);
  const { requestDecision, navigationGuardDialog } = useDraftNavigationGuard();
  const activePage = useMemo(() => getSettingsPage(activePageId), [activePageId]);

  /**
   * Task 12.16: unlike the draft guard above, this genuinely is rendered (every nav entry's own
   * bounded status dot), so it lives in real state, not a ref. Keyed by page id rather than one
   * active-page slot because a `draft-only` page (task 12.17) keeps reporting while backgrounded
   * -- its entry should keep flagging itself while a different page is on screen.
   */
  const [pageStatuses, setPageStatuses] = useState<Partial<Record<SettingsPageId, SettingsPageStatus>>>({});
  const handlePageStatusChange = useCallback((pageId: SettingsPageId, status: SettingsPageStatus | null) => {
    setPageStatuses((current) => {
      if (status !== null) return { ...current, [pageId]: status };
      if (!(pageId in current)) return current;
      const next = { ...current };
      delete next[pageId];
      return next;
    });
  }, []);
  // One stable callback per page id, computed once (`settingsPages` is the static module-level
  // list) -- a fresh closure built inline in the render loop below would change identity every
  // shell render, and a reporting page's own effect depends on that identity, which is exactly
  // the report-render-report loop the draft guard's own ref comment above already had to avoid.
  const statusReporters = useMemo(
    () => Object.fromEntries(
      settingsPages.map((page) => [page.id, (status: SettingsPageStatus | null) => handlePageStatusChange(page.id, status)]),
    ) as Record<SettingsPageId, (status: SettingsPageStatus | null) => void>,
    [handlePageStatusChange],
  );

  // A page reports its own guard through `onDraftStateChange`; clear it on every page switch so a
  // stale guard from the page just left can't outlive it (the newly active page re-reports its
  // own state, if any, once mounted/visible). Redundant with the reporting page's own
  // active-prop-changing cleanup, kept anyway as a cheap, explicit backstop.
  useEffect(() => { draftGuardRef.current = null; }, [activePageId]);

  /** Task 12.12: shell-coordinated Save/Discard/Stay instead of each page building its own
   *  blocking dialog. `proceed` only runs once the user has chosen to actually leave. */
  const guardedLeave = useCallback(async (proceed: () => void) => {
    const guard = draftGuardRef.current;
    if (!guard) { proceed(); return; }
    const outcome = await requestDecision({
      canSave: guard.canSave,
      dirtyCount: guard.dirtyCount,
      title: t("draftNavigationGuard.title"),
    });
    if (outcome === "stay") return;
    if (outcome === "save") {
      try {
        await guard.save();
      } catch {
        // Save failed -- stay rather than navigate away as though it had succeeded. The page's
        // own error surface (already shown next to its own Save control) explains what happened.
        return;
      }
    } else {
      guard.discard();
    }
    proceed();
  }, [requestDecision, t]);

  const handleSelectPage = useCallback((pageId: SettingsPageId, target?: SettingsNavigationTarget) => {
    function proceed() {
      setVisitedPages((current) => new Set(current).add(pageId));
      setActivePageId(pageId);
      setNavigationTarget(target ?? null);
      setSearchTerm("");
    }
    // A `draft-only`/`always` page keeps its own state across an inter-page switch on its own
    // (task 12.17) -- only a page that would actually unmount (`keepAlive: "never"`) needs
    // interrupting here. Reselecting the already-active page is never a real departure.
    if (pageId !== activePageId && draftGuardRef.current && SETTINGS_PAGE_LIFECYCLE[activePageId].keepAlive === "never") {
      void guardedLeave(proceed);
      return;
    }
    proceed();
  }, [activePageId, guardedLeave]);

  // Applied once per URL change, never re-asserted: the URL does not update on sidebar clicks, so
  // re-running on activePageId would snap every in-shell navigation back to the deep-linked page.
  // The initial page already honoured the URL (and initialNavigationTarget carries the deep-linked
  // sub-target), so a requested page that is already active is recorded without a navigation that
  // would wipe that target.
  const appliedRequestedPage = useRef<string | null>(requestedPage);
  useEffect(() => {
    if (
      requestedPage
      && appliedRequestedPage.current !== requestedPage
      && settingsPages.some((page) => page.id === requestedPage)
    ) {
      appliedRequestedPage.current = requestedPage;
      handleSelectPage(requestedPage as SettingsPageId);
    }
    // Intentionally excludes handleSelectPage, whose identity changes on every activePageId
    // change, which would reintroduce the snap-back bug this effect exists to prevent.
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [requestedPage]);

  // Leaving Settings entirely unmounts the whole shell regardless of any individual page's
  // lifecycle policy, so this is always guarded when a draft is reported -- unlike inter-page
  // switches, no `keepAlive` value protects a component that is about to be unrouted.
  const guardedOnReturn = onReturn ? () => { void guardedLeave(onReturn); } : undefined;

  // Task 12.6: a field result also needs the target page loaded before its anchor exists to
  // scroll to -- `useSettingsAnchorHighlight` itself polls for that, this only decides *which*
  // anchor (if any) to wait for once `handleSelectPage` has made the target page active.
  function handleSelectSearchResult(result: SettingsSearchResult) {
    handleSelectPage(result.page.id);
    setPendingAnchorId(result.entry.kind === "field" ? (result.entry.anchorId ?? null) : null);
  }
  useSettingsAnchorHighlight(pendingAnchorId, () => setPendingAnchorId(null));

  return (
    <main className="flex h-screen min-h-0 flex-col overflow-hidden bg-muted/40 text-foreground">
      {navigationGuardDialog}
      <SettingsTopBar
        activePage={activePage}
        onReturn={guardedOnReturn}
        onSearchTermChange={setSearchTerm}
        onSelectSearchResult={handleSelectSearchResult}
        searchIndex={settingsSearchIndex}
        searchTerm={searchTerm}
      />
      <div className="grid min-h-0 flex-1 grid-rows-[auto_minmax(0,1fr)] gap-4 px-4 pb-4 pt-0 lg:grid-cols-[clamp(220px,18vw,280px)_minmax(0,1fr)] lg:grid-rows-1 lg:gap-5 lg:px-5 lg:pb-5">
        <SettingsSidebar activePageId={activePageId} onSelectPage={handleSelectPage} pageStatuses={pageStatuses} />
        <SettingsCompactNav activePageId={activePageId} onSelectPage={handleSelectPage} pageStatuses={pageStatuses} />
        <section className="min-h-0 min-w-0 overflow-hidden rounded-lg border border-border bg-background shadow-xs">
          {settingsPages.map((page) => {
            const isActivePage = page.id === activePageId;
            if (!shouldRenderPage(SETTINGS_PAGE_LIFECYCLE[page.id], isActivePage, visitedPages.has(page.id))) return null;
            const pageProps = {
              isActive: isActivePage,
              navigationTarget: isActivePage ? navigationTarget : null,
              onDraftStateChange: isActivePage ? handleDraftStateChange : undefined,
              onNavigate: handleSelectPage,
              onOpenSession,
              onReturn: guardedOnReturn,
              onStatusChange: statusReporters[page.id],
              searchTerm: isActivePage ? searchTerm : "",
            };
            return (
              <div className="h-full overflow-y-auto" hidden={!isActivePage} key={page.id}>
                <div className="mx-auto w-full max-w-[1680px] px-5 py-5 sm:px-6 lg:px-8 xl:px-10">
                  <LazyFeature componentProps={pageProps} loader={page.loader} />
                </div>
              </div>
            );
          })}
        </section>
      </div>
    </main>
  );
}
