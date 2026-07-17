import { useMemo, useState } from "react";
import { defaultSettingsPageId, getSettingsPage, settingsPages, type SettingsPageId } from "./settings-pages";
import { SettingsSidebar } from "./settings-sidebar";
import { SettingsTopBar } from "./settings-topbar";

export function SettingsShell({ onReturn }: { onReturn?: () => void }) {
  const [activePageId, setActivePageId] = useState<SettingsPageId>(defaultSettingsPageId);
  const [searchTerm, setSearchTerm] = useState("");
  const activePage = useMemo(() => getSettingsPage(activePageId), [activePageId]);

  function handleSelectPage(pageId: SettingsPageId) {
    setActivePageId(pageId);
    setSearchTerm("");
  }

  return (
    <main className="flex h-screen min-h-0 flex-col overflow-hidden bg-background text-foreground">
      <div className="pointer-events-none fixed inset-0 opacity-[0.025] [background-image:linear-gradient(hsl(var(--primary))_1px,transparent_1px),linear-gradient(90deg,hsl(var(--primary))_1px,transparent_1px)] [background-size:96px_96px]" />
      <SettingsTopBar activePage={activePage} onReturn={onReturn} onSearchTermChange={setSearchTerm} searchTerm={searchTerm} />
      <div className="relative grid min-h-0 flex-1 gap-3 p-2 lg:grid-cols-[230px_minmax(0,1fr)]">
        <SettingsSidebar activePageId={activePageId} onSelectPage={handleSelectPage} />
        <section className="ucd-panel min-h-0 min-w-0 overflow-hidden rounded-lg p-3">
          {settingsPages.map((page) => {
            const Page = page.component;
            return (
              <div className="h-full overflow-y-auto pr-1" hidden={page.id !== activePageId} key={page.id}>
                <Page searchTerm={page.id === activePageId ? searchTerm : ""} />
              </div>
            );
          })}
        </section>
      </div>
    </main>
  );
}
