import { Sparkles } from "lucide-react";
import { useState } from "react";
import { useTranslation } from "react-i18next";
import { useSettings } from "../settings-provider";
import { PageHeader } from "./page-parts";
import { PersonalizationMemoryView } from "./personalization/memory-view";
import { PersonalizationInstructionsView } from "./personalization/instructions-view";
import { PersonalizationOverviewSection } from "./personalization/overview-section";
import { RuntimePreviewSection } from "./personalization/runtime-preview-section";
import {
  panelId,
  PersonalizationViewTabs,
  tabId,
  type PersonalizationView,
} from "./personalization/view-tabs";

export function PersonalizationPage({ onOpenSession }: { onOpenSession?: (sessionId: string) => void }) {
  const { t } = useTranslation();
  const { error } = useSettings();
  const [view, setView] = useState<PersonalizationView>("overview");

  return (
    <div className="mx-auto max-w-[1040px] space-y-5 pb-8">
      <PageHeader description={t("personalization.description")} icon={Sparkles} title={t("personalization.title")} />
      {error ? <div className="rounded-md border p-3 text-sm ucd-status-danger" role="alert">{error}</div> : null}
      <PersonalizationViewTabs onSelect={setView} view={view} />
      {/* Only the selected view mounts: the Memory view issues its own queries, and keeping all
          four mounted would fetch on every visit to the page regardless of what the user opened.
          The panel is a tab stop of its own, so arrowing to a destination and pressing Tab lands
          in it rather than skipping past the whole thing. */}
      <div
        aria-labelledby={tabId(view)}
        className="grid gap-5 focus-visible:outline-2 focus-visible:outline-offset-2"
        id={panelId(view)}
        role="tabpanel"
        tabIndex={0}
      >
        {view === "overview" ? <PersonalizationOverviewSection /> : null}
        {view === "instructions" ? <PersonalizationInstructionsView /> : null}
        {view === "memory" ? <PersonalizationMemoryView onOpenSession={onOpenSession} /> : null}
        {view === "runtimePreview" ? <RuntimePreviewSection /> : null}
      </div>
    </div>
  );
}
