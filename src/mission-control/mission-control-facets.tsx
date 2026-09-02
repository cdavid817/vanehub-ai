import { useTranslation } from "react-i18next";
import type { MissionControlFacet, MissionControlRunDetail } from "../types/mission-control";
import { OverviewFacet } from "./overview-facet";
import { UsageFacet } from "./usage-facet";

/**
 * Routes the selected detail facet to its real content, or to the placeholder every facet used to
 * render unconditionally before this pass. Only Overview and Usage have real content so far (16.8/
 * 16.9) — Timeline/Tools/Files/Review/Verification/Context/Logs stay on the placeholder, unchanged.
 *
 * Availability is re-checked here rather than trusted from the caller: the tab strip already
 * disables selecting an unavailable facet, but defaulting back to the placeholder for anything the
 * backend has not actually marked `"available"` keeps this component correct even if that ever
 * stops holding (e.g. a future `activeFacet` default that does not reset to "overview").
 */
export function MissionControlFacetPanel({ detail, facet }: { detail: MissionControlRunDetail; facet: MissionControlFacet }) {
  const { t } = useTranslation();
  const available = detail.facets.find((item) => item.facet === facet)?.state === "available";

  if (facet === "overview" && available) return <OverviewFacet run={detail.run} />;
  if (facet === "usage" && available) return <UsageFacet run={detail.run} />;

  return (
    <p className="mt-4 text-xs text-muted-foreground">
      {t("missionControl.facetSelected", { facet: t(`missionControl.facet.${facet}`) })} · {t("missionControl.lazyDetail")}
    </p>
  );
}
