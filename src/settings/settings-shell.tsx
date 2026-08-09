import { useEffect, useMemo, useState } from "react";
import { useSearchParams } from "react-router";
import { LazyFeature } from "../components/lazy-feature";
import { defaultSettingsPageId, getSettingsPage, settingsPages, type SettingsNavigationTarget, type SettingsPageId } from "./settings-pages";
import { SettingsSidebar } from "./settings-sidebar";
import { SettingsTopBar } from "./settings-topbar";

export function SettingsShell({
  initialNavigationTarget = null,
  initialPageId = defaultSettingsPageId,
  onReturn,
}: {
  initialNavigationTarget?: SettingsNavigationTarget | null;
  initialPageId?: SettingsPageId;
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

  return (
    <main className="flex h-screen min-h-0 flex-col overflow-hidden bg-muted/40 text-foreground">
      <SettingsTopBar activePage={activePage} onReturn={onReturn} onSearchTermChange={setSearchTerm} searchTerm={searchTerm} />
      <div className="grid min-h-0 flex-1 grid-rows-[auto_minmax(0,1fr)] gap-4 px-4 pb-4 pt-0 lg:grid-cols-[clamp(220px,18vw,280px)_minmax(0,1fr)] lg:grid-rows-1 lg:gap-5 lg:px-5 lg:pb-5">
        <SettingsSidebar activePageId={activePageId} onSelectPage={handleSelectPage} />
        <section className="min-h-0 min-w-0 overflow-hidden rounded-lg border border-border bg-background shadow-xs">
          {settingsPages.map((page) => {
            if (!visitedPages.has(page.id)) return null;
            const pageProps = {
              isActive: page.id === activePageId,
              navigationTarget: page.id === activePageId ? navigationTarget : null,
              onNavigate: handleSelectPage,
              searchTerm: page.id === activePageId ? searchTerm : "",
            };
            return (
              <div className="h-full overflow-y-auto" hidden={page.id !== activePageId} key={page.id}>
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
