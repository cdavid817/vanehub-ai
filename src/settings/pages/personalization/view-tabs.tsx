import { useRef } from "react";
import { useTranslation } from "react-i18next";

export const personalizationViews = ["overview", "instructions", "memory", "runtimePreview"] as const;

export type PersonalizationView = (typeof personalizationViews)[number];

export function tabId(view: PersonalizationView): string {
  return `personalization-tab-${view}`;
}

export function panelId(view: PersonalizationView): string {
  return `personalization-panel-${view}`;
}

/**
 * The four destinations inside AI Personalization.
 *
 * A roving tabindex rather than four tab stops: a tablist is one stop, and arrow keys move within
 * it. Four stops would make Tab walk through every destination before reaching the panel, which is
 * the thing the user actually came to use.
 */
export function PersonalizationViewTabs({
  onSelect,
  view,
}: {
  onSelect: (view: PersonalizationView) => void;
  view: PersonalizationView;
}) {
  const { t } = useTranslation();
  const list = useRef<HTMLDivElement>(null);

  function move(offset: number) {
    const index = personalizationViews.indexOf(view);
    const next = personalizationViews[(index + offset + personalizationViews.length) % personalizationViews.length];
    onSelect(next);
    // Selection follows focus, so focus has to follow selection too, or the arrow key changes the
    // panel while the keyboard stays behind on the old tab.
    list.current?.querySelector<HTMLButtonElement>(`#${tabId(next)}`)?.focus();
  }

  function onKeyDown(event: React.KeyboardEvent<HTMLDivElement>) {
    if (event.key === "ArrowRight" || event.key === "ArrowDown") {
      event.preventDefault();
      move(1);
    } else if (event.key === "ArrowLeft" || event.key === "ArrowUp") {
      event.preventDefault();
      move(-1);
    } else if (event.key === "Home") {
      event.preventDefault();
      move(-personalizationViews.indexOf(view));
    } else if (event.key === "End") {
      event.preventDefault();
      move(personalizationViews.length - 1 - personalizationViews.indexOf(view));
    }
  }

  return (
    <div
      aria-label={t("personalization.views.label")}
      className="flex max-w-full gap-1 overflow-x-auto rounded-lg border border-border bg-muted/25 p-1"
      onKeyDown={onKeyDown}
      ref={list}
      role="tablist"
    >
      {personalizationViews.map((candidate) => (
        <button
          aria-controls={panelId(candidate)}
          aria-selected={view === candidate}
          className={`min-h-9 whitespace-nowrap rounded-md px-3 text-sm font-medium transition-colors ${view === candidate ? "bg-background text-foreground shadow-xs" : "text-muted-foreground hover:bg-background/70 hover:text-foreground"}`}
          data-testid={`personalization-view-tab-${candidate}`}
          id={tabId(candidate)}
          key={candidate}
          onClick={() => onSelect(candidate)}
          role="tab"
          tabIndex={view === candidate ? 0 : -1}
          type="button"
        >
          {t(`personalization.views.${candidate}`)}
        </button>
      ))}
    </div>
  );
}
