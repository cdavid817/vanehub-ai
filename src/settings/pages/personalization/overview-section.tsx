import { LayoutGrid } from "lucide-react";
import { useTranslation } from "react-i18next";
import { SectionPanel } from "../page-parts";

/**
 * The Overview destination.
 *
 * Task 10.1 establishes the destination; task 10.3 fills it with the effective-source cards and the
 * dynamic Agent list, and task 11.7 adds maintenance and health. Until then it states what belongs
 * here rather than rendering a card built from data the page does not yet load -- an Overview that
 * showed a number nothing computed would be worse than one that is honestly empty.
 */
export function PersonalizationOverviewSection() {
  const { t } = useTranslation();

  return (
    <SectionPanel
      description={t("personalization.overview.description")}
      icon={LayoutGrid}
      title={t("personalization.overview.title")}
    >
      <p className="text-sm leading-6 text-muted-foreground" data-testid="personalization-overview-empty">
        {t("personalization.overview.empty")}
      </p>
    </SectionPanel>
  );
}
