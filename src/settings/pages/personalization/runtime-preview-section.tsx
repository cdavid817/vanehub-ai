import { Eye } from "lucide-react";
import { useTranslation } from "react-i18next";
import { SectionPanel } from "../page-parts";

/**
 * The Runtime Preview destination.
 *
 * Task 10.1 establishes the destination; task 11.8 adds the Agent, workspace and session-mode
 * inputs and the provenance/exclusion output. A preview needs a selection to be about, and the
 * selection controls arrive with it -- rendering one for a guessed Agent would show a resolution
 * no session ever had.
 */
export function RuntimePreviewSection() {
  const { t } = useTranslation();

  return (
    <SectionPanel
      description={t("personalization.runtimePreview.description")}
      icon={Eye}
      title={t("personalization.runtimePreview.title")}
    >
      <p className="text-sm leading-6 text-muted-foreground" data-testid="personalization-runtime-preview-empty">
        {t("personalization.runtimePreview.empty")}
      </p>
      <p className="mt-3 text-xs leading-5 text-muted-foreground">
        {t("personalization.runtimePreview.cliCompaction")}
      </p>
    </SectionPanel>
  );
}
