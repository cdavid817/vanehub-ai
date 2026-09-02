import { useTranslation } from "react-i18next";
import type { AgentRegistryEntry } from "../types/agent";
import type {
  MissionControlAction, MissionControlFacet, MissionControlFacetState,
  MissionControlRunDetail, MissionControlRunSummary,
} from "../types/mission-control";
import type { MutationState } from "../ui/async/mutation-state";
import { MissionControlFacetPanel } from "./mission-control-facets";
import { RunCard } from "./mission-control-run-card";

const facets = ["overview", "timeline", "tools", "files", "review", "verification", "context", "usage", "logs"] as const;

export interface FacetTabListProps {
  activeFacet: MissionControlFacet;
  availability: (facet: MissionControlFacet) => MissionControlFacetState;
  onSelect: (facet: MissionControlFacet) => void;
}

/** 16.3's own "section navigation" (design.md Decision 13's own wording for this exact strip): the
 *  nine-evidence-type tab list, unchanged from the original inline version. 16.8 (a separate,
 *  larger, unstarted task) owns actually redesigning this into the readable navigation plus compact
 *  selector fallback Decision 13 describes -- this pass only relocates the existing markup. */
function FacetTabList({ activeFacet, availability, onSelect }: FacetTabListProps) {
  const { t } = useTranslation();
  return (
    <div className="mt-3 flex gap-1 overflow-x-auto" role="tablist">
      {facets.map((facet) => {
        const state = availability(facet);
        return (
          <button
            aria-disabled={state !== "available"}
            aria-selected={activeFacet === facet}
            className="shrink-0 rounded-md border border-input px-2 py-1 text-xs disabled:opacity-50"
            disabled={state !== "available"}
            key={facet}
            onClick={() => onSelect(facet)}
            role="tab"
            type="button"
          >
            {t(`missionControl.facet.${facet}`)}{state === "available" ? null : ` · ${t(`missionControl.availability.${state}`)}`}
          </button>
        );
      })}
    </div>
  );
}

export interface MissionControlDetailPanelProps {
  activeFacet: MissionControlFacet;
  agents: readonly AgentRegistryEntry[];
  mutation?: MutationState;
  onAct: (run: MissionControlRunSummary, action: MissionControlAction) => void;
  onDismissError: () => void;
  onInspect: (run: MissionControlRunSummary) => void;
  onSelectFacet: (facet: MissionControlFacet) => void;
  selected: MissionControlRunDetail | null;
}

/** 16.3's own "detail" region: the selected Run's own card, section navigation, and the (untouched,
 *  16.9-owned) facet content panel. */
export function MissionControlDetailPanel({ activeFacet, agents, mutation, onAct, onDismissError, onInspect, onSelectFacet, selected }: MissionControlDetailPanelProps) {
  const { t } = useTranslation();
  if (!selected) return <p className="text-sm text-muted-foreground">{t("missionControl.selectRun")}</p>;
  const availability = (facet: MissionControlFacet) => selected.facets.find((item) => item.facet === facet)?.state ?? "unavailable";
  return (
    <>
      <RunCard agents={agents} mutation={mutation} onAct={onAct} onDismissError={() => onDismissError()} onInspect={onInspect} run={selected.run} />
      <FacetTabList activeFacet={activeFacet} availability={availability} onSelect={onSelectFacet} />
      <MissionControlFacetPanel detail={selected} facet={activeFacet} />
    </>
  );
}
