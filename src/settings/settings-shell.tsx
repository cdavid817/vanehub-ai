import { useEffect, useMemo, useState } from "react";
import { useSearchParams } from "react-router";
import { LazyFeature } from "../components/lazy-feature";
import { useSettingsAnchorHighlight } from "../hooks/use-settings-anchor-highlight";
import { shouldRenderPage } from "../ui/page-lifecycle/page-lifecycle-policy";
import { SETTINGS_PAGE_LIFECYCLE } from "./settings-page-lifecycle";
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
  const activePage = useMemo(() => getSettingsPage(activePageId), [activePageId]);

  useEffect(() => {
    if (requestedPage && settingsPages.some((page) => page.id === requestedPage)) handleSelectPage(requestedPage as SettingsPageId);
  }, [requestedPage]);

  function handleSelectPage(pageId: SettingsPageId, target?: SettingsNavigationTarget) {
    setVisitedPages((current) => new Set(current).add(pageId));
    setActivePageId(pageId);
    setNavigationTarget(target ?? null);
    setSearchTerm("");
  }

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
      <SettingsTopBar
        activePage={activePage}
        onReturn={onReturn}
        onSearchTermChange={setSearchTerm}
        onSelectSearchResult={handleSelectSearchResult}
        searchIndex={settingsSearchIndex}
        searchTerm={searchTerm}
      />
      <div className="grid min-h-0 flex-1 grid-rows-[auto_minmax(0,1fr)] gap-4 px-4 pb-4 pt-0 lg:grid-cols-[clamp(220px,18vw,280px)_minmax(0,1fr)] lg:grid-rows-1 lg:gap-5 lg:px-5 lg:pb-5">
        <SettingsSidebar activePageId={activePageId} onSelectPage={handleSelectPage} />
        <SettingsCompactNav activePageId={activePageId} onSelectPage={handleSelectPage} />
        <section className="min-h-0 min-w-0 overflow-hidden rounded-lg border border-border bg-background shadow-xs">
          {settingsPages.map((page) => {
            const isActivePage = page.id === activePageId;
            if (!shouldRenderPage(SETTINGS_PAGE_LIFECYCLE[page.id], isActivePage, visitedPages.has(page.id))) return null;
            const pageProps = {
              isActive: isActivePage,
              navigationTarget: isActivePage ? navigationTarget : null,
              onNavigate: handleSelectPage,
              onOpenSession,
              onReturn,
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
