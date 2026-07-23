import { useMemo, useState } from "react";
import { LazyFeature } from "../components/lazy-feature";
import { defaultSettingsPageId, getSettingsPage, settingsPages, type SettingsPageId } from "./settings-pages";
import { SettingsSidebar } from "./settings-sidebar";
import { SettingsTopBar } from "./settings-topbar";

export function SettingsShell({ onReturn }: { onReturn?: () => void }) {
  const [activePageId, setActivePageId] = useState<SettingsPageId>(defaultSettingsPageId);
  const [visitedLazyPages, setVisitedLazyPages] = useState<Set<SettingsPageId>>(() => new Set());
  const [searchTerm, setSearchTerm] = useState("");
  const activePage = useMemo(() => getSettingsPage(activePageId), [activePageId]);

  function handleSelectPage(pageId: SettingsPageId) {
    if (getSettingsPage(pageId).loader) {
      setVisitedLazyPages((current) => new Set(current).add(pageId));
    }
    setActivePageId(pageId);
    setSearchTerm("");
  }

  return (
    <main className="flex h-screen min-h-0 flex-col overflow-hidden bg-muted/40 text-foreground">
      <SettingsTopBar activePage={activePage} onReturn={onReturn} onSearchTermChange={setSearchTerm} searchTerm={searchTerm} />
      <div className="grid min-h-0 flex-1 grid-rows-[auto_minmax(0,1fr)] gap-4 px-4 pb-4 pt-0 lg:grid-cols-[clamp(220px,18vw,280px)_minmax(0,1fr)] lg:grid-rows-1 lg:gap-5 lg:px-5 lg:pb-5">
        <SettingsSidebar activePageId={activePageId} onSelectPage={handleSelectPage} />
        <section className="min-h-0 min-w-0 overflow-hidden rounded-lg border border-border bg-background shadow-sm">
          {settingsPages.map((page) => {
            const Page = page.component;
            if (!Page && (!page.loader || !visitedLazyPages.has(page.id))) return null;
            const pageProps = {
              onNavigate: handleSelectPage,
              searchTerm: page.id === activePageId ? searchTerm : "",
            };
            return (
              <div className="h-full overflow-y-auto" hidden={page.id !== activePageId} key={page.id}>
                <div className="mx-auto w-full max-w-[1680px] px-5 py-5 sm:px-6 lg:px-8 xl:px-10">
                  {Page ? <Page {...pageProps} /> : page.loader ? (
                    <LazyFeature componentProps={pageProps} loader={page.loader} />
                  ) : null}
                </div>
              </div>
            );
          })}
        </section>
      </div>
    </main>
  );
}
