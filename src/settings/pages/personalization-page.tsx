import { Sparkles } from "lucide-react";
import { useState } from "react";
import { useTranslation } from "react-i18next";
import { useSettings } from "../settings-provider";
import { PageHeader } from "./page-parts";
import { AgentMemorySection } from "./personalization/agent-memory-section";
import { CustomInstructionsSection } from "./personalization/custom-instructions-section";
import { PersonalizationOverviewSection } from "./personalization/overview-section";
import { RuntimePreviewSection } from "./personalization/runtime-preview-section";
import { PersonalizationViewTabs, type PersonalizationView } from "./personalization/view-tabs";

export function PersonalizationPage() {
  const { t } = useTranslation();
  const { error } = useSettings();
  const [view, setView] = useState<PersonalizationView>("overview");

  return (
    <div className="mx-auto max-w-[1040px] space-y-5 pb-8">
      <PageHeader description={t("personalization.description")} icon={Sparkles} title={t("personalization.title")} />
      {error ? <div className="rounded-md border p-3 text-sm ucd-status-danger" role="alert">{error}</div> : null}
      <PersonalizationViewTabs onSelect={setView} view={view} />
      {/* Only the selected view mounts: the Memory view issues its own queries, and keeping all
          four mounted would fetch on every visit to the page regardless of what the user opened. */}
      <div className="grid gap-5">
        {view === "overview" ? <PersonalizationOverviewSection /> : null}
        {view === "instructions" ? <CustomInstructionsSection /> : null}
        {view === "memory" ? <AgentMemorySection /> : null}
        {view === "runtimePreview" ? <RuntimePreviewSection /> : null}
      </div>
    </div>
  );
}
