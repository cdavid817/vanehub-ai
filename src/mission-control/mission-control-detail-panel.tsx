import { useTranslation } from "react-i18next";
import type { AgentRegistryEntry } from "../types/agent";
import type {
  MissionControlAction, MissionControlFacet,
  MissionControlRunDetail, MissionControlRunSummary,
} from "../types/mission-control";
import type { MutationState } from "../ui/async/mutation-state";
import { MissionControlFacetPanel } from "./mission-control-facets";
import { MissionControlSectionNav } from "./mission-control-section-nav";
import { RunCard } from "./mission-control-run-card";

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

/** 16.3's own "detail" region: the selected Run's own card, section navigation (16.8:
 *  `MissionControlSectionNav`), and the facet content panel (16.9-16.11: `MissionControlFacetPanel`). */
export function MissionControlDetailPanel({ activeFacet, agents, mutation, onAct, onDismissError, onInspect, onSelectFacet, selected }: MissionControlDetailPanelProps) {
  const { t } = useTranslation();
  if (!selected) return <p className="text-sm text-muted-foreground">{t("missionControl.selectRun")}</p>;
  const availability = (facet: MissionControlFacet) => selected.facets.find((item) => item.facet === facet)?.state ?? "unavailable";
  return (
    <>
      <RunCard agents={agents} mutation={mutation} onAct={onAct} onDismissError={() => onDismissError()} onInspect={onInspect} run={selected.run} />
      <MissionControlSectionNav activeFacet={activeFacet} availability={availability} onSelect={onSelectFacet} />
      <MissionControlFacetPanel detail={selected} facet={activeFacet} />
    </>
  );
}
