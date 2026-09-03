import { ListChecks, Search, TriangleAlert } from "lucide-react";
import { useState } from "react";
import { useTranslation } from "react-i18next";
import { cn } from "../lib/utils";
import type { AgentRegistryEntry } from "../types/agent";
import {
  collectEvaluationCapabilityTags,
  EVALUATION_AGENT_FILTERS_DEFAULT,
  EVALUATION_AGENT_STATUSES,
  filterEvaluationAgents,
  isEvaluationAgentIncompatible,
  MAX_EVALUATION_AGENTS,
  type EvaluationAgentFilters,
} from "./evaluation-agent-filters";

export interface EvaluationAgentSelectorProps {
  agents: AgentRegistryEntry[];
  selectedIds: string[];
  onToggle: (agentId: string) => void;
  /** "Select visible" (18.5): replaces the selection with exactly the Agents `filterEvaluationAgents`
   *  currently narrows to -- not a merge with whatever was selected before. */
  onSelectVisible: (agentIds: string[]) => void;
}

const selectClass = "h-9 rounded-md border border-input bg-background px-2 text-sm";

/**
 * 18.5: searchable Agent selection with status/capability filters, select-visible, a selected
 * summary, and incompatibility reasons. Filtering only ever narrows what is *visible*: an Agent
 * flagged incompatible is never hidden by that flag alone, only by an explicit status filter the
 * reader chose, so a real reason (`isEvaluationAgentIncompatible`'s own doc comment) stays
 * discoverable rather than silently dropped.
 */
export function EvaluationAgentSelector({ agents, onSelectVisible, onToggle, selectedIds }: EvaluationAgentSelectorProps) {
  const { t } = useTranslation();
  const [filters, setFilters] = useState<EvaluationAgentFilters>(EVALUATION_AGENT_FILTERS_DEFAULT);
  const capabilities = collectEvaluationCapabilityTags(agents);
  const visible = filterEvaluationAgents(agents, filters);
  const atCapacity = selectedIds.length >= MAX_EVALUATION_AGENTS;

  return (
    <div className="grid gap-3">
      <div className="grid gap-2 sm:grid-cols-[1fr_auto_auto]">
        <div className="relative">
          <Search aria-hidden="true" className="absolute left-2.5 top-1/2 h-4 w-4 -translate-y-1/2 text-muted-foreground" />
          <input
            aria-label={t("evaluation.agentSelection.searchLabel")}
            className="h-9 w-full rounded-md border border-input bg-background pl-8 pr-2 text-sm"
            onChange={(event) => setFilters((current) => ({ ...current, query: event.target.value }))}
            placeholder={t("evaluation.agentSelection.searchPlaceholder")}
            value={filters.query}
          />
        </div>
        <select
          aria-label={t("evaluation.agentSelection.statusLabel")}
          className={selectClass}
          onChange={(event) => setFilters((current) => ({ ...current, status: event.target.value as EvaluationAgentFilters["status"] }))}
          value={filters.status}
        >
          <option value="all">{t("evaluation.agentSelection.statusAll")}</option>
          {EVALUATION_AGENT_STATUSES.map((state) => <option key={state} value={state}>{t(`evaluation.agentStatus.${state}`)}</option>)}
        </select>
        <select
          aria-label={t("evaluation.agentSelection.capabilityLabel")}
          className={selectClass}
          onChange={(event) => setFilters((current) => ({ ...current, capability: event.target.value }))}
          value={filters.capability}
        >
          <option value="all">{t("evaluation.agentSelection.capabilityAll")}</option>
          {capabilities.map((tag) => <option key={tag} value={tag}>{tag}</option>)}
        </select>
      </div>

      <div className="flex flex-wrap items-center justify-between gap-2 text-xs text-muted-foreground">
        <span>{t("evaluation.agentSelection.resultCount", { count: visible.length, total: agents.length })}</span>
        <div className="flex items-center gap-3">
          <span data-testid="evaluation-selected-summary">{t("evaluation.selectedCount", { count: selectedIds.length })}</span>
          <button
            className="flex items-center gap-1 rounded-md border border-input px-2 py-1 text-xs disabled:cursor-not-allowed disabled:opacity-50"
            data-testid="evaluation-select-visible"
            disabled={visible.length === 0}
            onClick={() => onSelectVisible(visible.map((agent) => agent.id))}
            type="button"
          >
            <ListChecks aria-hidden="true" className="h-3.5 w-3.5" />
            {t("evaluation.agentSelection.selectVisible")}
          </button>
        </div>
      </div>
      <p className="text-xs text-muted-foreground">{t("evaluation.agentSelection.maxAgents")}</p>

      <ul aria-label={t("evaluation.agents")} className="grid max-h-72 gap-2 overflow-y-auto">
        {visible.map((agent) => {
          const selected = selectedIds.includes(agent.id);
          const incompatible = isEvaluationAgentIncompatible(agent);
          return (
            <li key={agent.id}>
              <label
                className={cn(
                  "flex items-start gap-2 rounded-md border border-border p-2 text-sm",
                  selected && "border-primary bg-[hsl(var(--nav-active-soft))] shadow-[0_0_0_1px_hsl(var(--primary))]",
                )}
              >
                <input
                  checked={selected}
                  className="mt-0.5"
                  data-testid={`evaluation-agent-${agent.id}`}
                  disabled={!selected && atCapacity}
                  onChange={() => onToggle(agent.id)}
                  type="checkbox"
                />
                <span className="min-w-0 flex-1">
                  <span className="flex flex-wrap items-center gap-1.5">
                    <span className="font-medium">{agent.displayName}</span>
                    <span className="text-xs text-muted-foreground">{agent.id} · {agent.provider}</span>
                  </span>
                  {agent.capabilityTags.length > 0 ? (
                    <span className="mt-1 flex flex-wrap gap-1">
                      {agent.capabilityTags.map((tag) => (
                        <span className="rounded bg-muted px-1.5 py-0.5 text-[0.6875rem] text-muted-foreground" key={tag}>{tag}</span>
                      ))}
                    </span>
                  ) : null}
                  {incompatible ? (
                    <span className="mt-1 flex items-start gap-1 text-xs text-[hsl(var(--warning))]" data-testid={`evaluation-agent-${agent.id}-reason`}>
                      <TriangleAlert aria-hidden="true" className="mt-0.5 h-3.5 w-3.5 shrink-0" />
                      <span>
                        {t(`evaluation.agentStatus.${agent.availabilityState}`)}
                        {agent.unavailableReason ? <span className="block text-muted-foreground" title={agent.unavailableReason}>{agent.unavailableReason}</span> : null}
                      </span>
                    </span>
                  ) : null}
                </span>
              </label>
            </li>
          );
        })}
        {visible.length === 0 ? <li className="rounded-md border border-dashed border-border p-4 text-center text-xs text-muted-foreground">{t("evaluation.agentSelection.empty")}</li> : null}
      </ul>
    </div>
  );
}
