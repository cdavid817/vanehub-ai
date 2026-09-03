import { useEffect, useRef, useState, type RefObject } from "react";
import { useTranslation } from "react-i18next";
import type { MissionControlFacet, MissionControlFacetState } from "../types/mission-control";
import { useTabList } from "../ui/runtime-panel/use-tab-list";

const FACETS = ["overview", "timeline", "tools", "files", "review", "verification", "context", "usage", "logs"] as const;
const TAB_ITEMS = FACETS.map((facet) => ({ id: facet }));

/**
 * Below this container width, nine readable tab labels cannot fit on one line without clipping —
 * mirrors `useTableCompactMode`'s own "narrow sidebar" reasoning (`src/ui/data-table/use-table-
 * compact-mode.ts`): a container-width check, not a page-level `useMediaQuery` breakpoint, because
 * this strip lives inside Mission Control's own `aside` column (`minmax(280px, 1fr)`), whose real
 * width tracks that column's own share of the layout, not the viewport as a whole. Matches that
 * same file's own threshold value for the same class of problem (interactive content embedded in a
 * narrow sidebar), since no live-rendered measurement is available to pick a more exact number.
 */
const COMPACT_MAX_WIDTH = 640;

function useSectionNavCompactMode(containerRef: RefObject<HTMLElement | null>): boolean {
  const [compact, setCompact] = useState(false);

  useEffect(() => {
    const element = containerRef.current;
    if (!element) return;
    const observer = new ResizeObserver((entries) => {
      const width = entries[0]?.contentRect.width ?? element.clientWidth;
      setCompact(width < COMPACT_MAX_WIDTH);
    });
    observer.observe(element);
    return () => observer.disconnect();
  }, [containerRef]);

  return compact;
}

export interface MissionControlSectionNavProps {
  activeFacet: MissionControlFacet;
  availability: (facet: MissionControlFacet) => MissionControlFacetState;
  onSelect: (facet: MissionControlFacet) => void;
}

/**
 * The readable, horizontally-scrollable `role="tablist"` variant — unchanged in spirit from the
 * original inline `FacetTabList` it replaces, now with roving-tabindex keyboard navigation
 * (Left/Right/Home/End), matching the same WAI-ARIA tab pattern `RuntimePanel` already established
 * for its own tablist (`src/ui/runtime-panel/RuntimePanel.tsx`) via the same shared `useTabList`.
 */
function ReadableSectionNav({ activeFacet, availability, onSelect }: MissionControlSectionNavProps) {
  const { t } = useTranslation();
  // useTabList's own onActiveTabChange is typed `(id: string) => void` (it is generic over any
  // `{id: string}` shape), narrower than `onSelect`'s `MissionControlFacet` -- routed through the
  // same validated lookup CompactSectionNav's own onChange below uses, rather than an unchecked cast.
  const handleActiveTabChange = (id: string) => {
    const next = FACETS.find((facet) => facet === id);
    if (next) onSelect(next);
  };
  const { handleKeyDown, registerTabRef } = useTabList(TAB_ITEMS, activeFacet, handleActiveTabChange);
  return (
    <div
      aria-label={t("missionControl.sectionNav.label")}
      className="flex gap-1 overflow-x-auto"
      onKeyDown={handleKeyDown}
      role="tablist"
    >
      {FACETS.map((facet) => {
        const state = availability(facet);
        return (
          <button
            aria-disabled={state !== "available"}
            aria-selected={activeFacet === facet}
            className="shrink-0 rounded-md border border-input px-2 py-1 text-xs disabled:opacity-50"
            disabled={state !== "available"}
            key={facet}
            onClick={() => onSelect(facet)}
            ref={registerTabRef(facet)}
            role="tab"
            tabIndex={activeFacet === facet ? 0 : -1}
            type="button"
          >
            {t(`missionControl.facet.${facet}`)}{state === "available" ? null : ` · ${t(`missionControl.availability.${state}`)}`}
          </button>
        );
      })}
    </div>
  );
}

/**
 * The narrow-container fallback: one native `<select>` in place of nine tab buttons — design.md
 * Decision 13's own "宽度不足时使用 Select" (use a Select when width is insufficient). Reuses the
 * exact same facet/availability i18n keys the readable variant does, so the two variants can never
 * drift apart in wording.
 */
function CompactSectionNav({ activeFacet, availability, onSelect }: MissionControlSectionNavProps) {
  const { t } = useTranslation();
  return (
    <select
      aria-label={t("missionControl.sectionNav.label")}
      className="h-8 w-full rounded-md border border-input bg-background px-2 text-xs"
      onChange={(event) => {
        const next = FACETS.find((facet) => facet === event.target.value);
        if (next) onSelect(next);
      }}
      value={activeFacet}
    >
      {FACETS.map((facet) => {
        const state = availability(facet);
        return (
          <option disabled={state !== "available"} key={facet} value={facet}>
            {t(`missionControl.facet.${facet}`)}{state === "available" ? "" : ` · ${t(`missionControl.availability.${state}`)}`}
          </option>
        );
      })}
    </select>
  );
}

/**
 * 16.8: exported so its own test can force each variant directly. jsdom defines no `ResizeObserver`
 * at all, so a component test can only ever observe this component's *default* (non-compact) state —
 * the same limitation `src/ui/data-table/DataTable.tsx` already has, solved there the same way: split
 * the ResizeObserver-driven measurement from a plain presentational component that takes `compact`
 * as a prop, and test that component directly.
 */
export function MissionControlSectionNavView({ compact, ...props }: MissionControlSectionNavProps & { compact: boolean }) {
  return compact ? <CompactSectionNav {...props} /> : <ReadableSectionNav {...props} />;
}

/**
 * 16.8: replaces the always-nine-visible-tabs `FacetTabList` (previously inline in
 * `mission-control-detail-panel.tsx`, styled with `overflow-x-auto` and nothing else for narrow
 * widths) with a real compact-selector fallback. Measures its own container rather than the
 * viewport — see `useSectionNavCompactMode`'s own doc comment for why.
 */
export function MissionControlSectionNav(props: MissionControlSectionNavProps) {
  const containerRef = useRef<HTMLDivElement>(null);
  const compact = useSectionNavCompactMode(containerRef);
  return (
    <div className="mt-3" data-testid="mission-control-section-nav" ref={containerRef}>
      <MissionControlSectionNavView {...props} compact={compact} />
    </div>
  );
}
