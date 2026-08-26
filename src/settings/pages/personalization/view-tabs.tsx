import { useTranslation } from "react-i18next";

export const personalizationViews = ["overview", "instructions", "memory", "runtimePreview"] as const;

export type PersonalizationView = (typeof personalizationViews)[number];

/**
 * The four destinations inside AI Personalization.
 *
 * Deliberately the same tablist the OnePiece configuration panel uses rather than a new primitive:
 * two switchers that look alike but behave differently is worse than one that is plain.
 */
export function PersonalizationViewTabs({
  onSelect,
  view,
}: {
  onSelect: (view: PersonalizationView) => void;
  view: PersonalizationView;
}) {
  const { t } = useTranslation();

  return (
    <div
      aria-label={t("personalization.views.label")}
      className="flex max-w-full gap-1 overflow-x-auto rounded-lg border border-border bg-muted/25 p-1"
      role="tablist"
    >
      {personalizationViews.map((candidate) => (
        <button
          aria-selected={view === candidate}
          className={`min-h-9 whitespace-nowrap rounded-md px-3 text-sm font-medium transition-colors ${view === candidate ? "bg-background text-foreground shadow-xs" : "text-muted-foreground hover:bg-background/70 hover:text-foreground"}`}
          data-testid={`personalization-view-tab-${candidate}`}
          key={candidate}
          onClick={() => onSelect(candidate)}
          role="tab"
          type="button"
        >
          {t(`personalization.views.${candidate}`)}
        </button>
      ))}
    </div>
  );
}
