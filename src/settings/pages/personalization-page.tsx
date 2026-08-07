import { Sparkles } from "lucide-react";
import { useTranslation } from "react-i18next";
import { useSettings } from "../settings-provider";
import { PageHeader } from "./page-parts";
import { CustomInstructionsSection } from "./personalization/custom-instructions-section";
import { AgentMemorySection } from "./personalization/agent-memory-section";

export function PersonalizationPage() {
  const { t } = useTranslation();
  const { error } = useSettings();

  return (
    <div className="mx-auto max-w-[1040px] space-y-5 pb-8">
      <PageHeader description={t("personalization.description")} icon={Sparkles} title={t("personalization.title")} />
      {error ? <div className="rounded-md border p-3 text-sm ucd-status-danger" role="alert">{error}</div> : null}
      <div className="grid gap-5">
        <CustomInstructionsSection />
        <AgentMemorySection />
      </div>
    </div>
  );
}
