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
      <SettingsTopBar activePage={activePage} onReturn={onReturn} onSearchTermChange={setSearchTerm} searchTerm={searchTerm} />
      <div className="grid min-h-0 flex-1 gap-4 p-2 lg:grid-cols-[230px_minmax(0,1fr)]">
        <SettingsSidebar activePageId={activePageId} onSelectPage={handleSelectPage} />
        <section className="min-h-0 min-w-0 overflow-hidden">
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
