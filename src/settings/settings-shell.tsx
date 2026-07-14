import { useMemo, useState } from "react";
import { defaultSettingsPageId, getSettingsPage, type SettingsPageId } from "./settings-pages";
import { SettingsSidebar } from "./settings-sidebar";
import { SettingsTopBar } from "./settings-topbar";

export function SettingsShell({ onReturn }: { onReturn?: () => void }) {
  const [activePageId, setActivePageId] = useState<SettingsPageId>(defaultSettingsPageId);
  const [searchTerm, setSearchTerm] = useState("");
  const activePage = useMemo(() => getSettingsPage(activePageId), [activePageId]);
  const ActivePage = activePage.component;

  function handleSelectPage(pageId: SettingsPageId) {
    setActivePageId(pageId);
    setSearchTerm("");
  }

  return (
    <main className="min-h-screen bg-background text-foreground">
      <SettingsTopBar activePage={activePage} onReturn={onReturn} onSearchTermChange={setSearchTerm} searchTerm={searchTerm} />
      <div className="grid gap-4 p-2 lg:grid-cols-[230px_minmax(0,1fr)]">
        <SettingsSidebar activePageId={activePageId} onSelectPage={handleSelectPage} />
        <section className="min-w-0">
          <ActivePage searchTerm={searchTerm} />
        </section>
      </div>
    </main>
  );
}
