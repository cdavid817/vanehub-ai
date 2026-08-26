import { useQuery } from "@tanstack/react-query";
import { Bot, FileText, Brain, Layers, LayoutGrid, LoaderCircle } from "lucide-react";
import { useTranslation } from "react-i18next";
import type { AgentService } from "../../../services/agent-service";
import { agentService as defaultAgentService } from "../../../services/runtime-agent-client";
import { SectionPanel, StatCard } from "../page-parts";
import { AgentOverviewList } from "./agent-overview-list";
import { agentOverviewRow, overviewTotals, overviewWarnings, type AgentOverviewRow } from "./overview-model";

/**
 * A resolution needs a session to be about, and the Overview is not about one.
 *
 * The id identifies the snapshot rather than selecting anything, so a fixed value gives every
 * Agent's row the same footing. It is deliberately not a real session id: reusing one would report
 * that session's mode and workspace as though they were the resting state.
 */
const OVERVIEW_SESSION_ID = "personalization-overview";

const overviewQueryKey = ["personalization", "overview"] as const;

async function loadOverview(service: AgentService): Promise<AgentOverviewRow[]> {
  const capabilities = await service.listPersonalizationAgentCapabilities();
  return Promise.all(
    capabilities.map(async (capability) => {
      const preview = await service.previewEffectivePersonalization({
        agentId: capability.agentId,
        sessionId: OVERVIEW_SESSION_ID,
      });
      return agentOverviewRow(capability, preview);
    }),
  );
}

export function PersonalizationOverviewSection({ service = defaultAgentService }: { service?: AgentService }) {
  const { t } = useTranslation();
  const rowsQuery = useQuery({ queryKey: overviewQueryKey, queryFn: () => loadOverview(service) });
  const policiesQuery = useQuery({
    queryKey: ["personalization", "policies"] as const,
    queryFn: () => service.listPersonalizationPolicies(),
  });

  const rows = rowsQuery.data ?? [];
  const totals = overviewTotals(rows, policiesQuery.data ?? []);
  const warnings = overviewWarnings(rows);
  const failure = rowsQuery.error ?? policiesQuery.error;

  return (
    <SectionPanel
      description={t("personalization.overview.description")}
      icon={LayoutGrid}
      title={t("personalization.overview.title")}
    >
      {failure ? (
        <div className="mb-4 rounded-md border p-3 text-sm ucd-status-danger" role="alert" data-testid="personalization-overview-error">
          {t("personalization.overview.loadFailed")}
        </div>
      ) : null}

      <div className="grid gap-3 sm:grid-cols-2 xl:grid-cols-4" data-testid="personalization-overview">
        <StatCard
          hint={t("personalization.overview.cards.agentsHint", { count: totals.agentsWithInstructions })}
          icon={Bot}
          label={t("personalization.overview.cards.agents")}
          value={String(totals.agents)}
        />
        <StatCard
          hint={t("personalization.overview.cards.instructionsHint", { count: totals.globalCharacters })}
          icon={FileText}
          label={t("personalization.overview.cards.instructions")}
          value={String(totals.agentsWithInstructions)}
        />
        <StatCard
          hint={t("personalization.overview.cards.memoryHint", { count: totals.extractionAgents })}
          icon={Brain}
          label={t("personalization.overview.cards.memory")}
          value={String(totals.memoryReadAgents)}
        />
        <StatCard
          hint={t("personalization.overview.cards.scopesHint")}
          icon={Layers}
          label={t("personalization.overview.cards.scopes")}
          value={String(totals.configuredScopes)}
        />
      </div>

      {warnings.length > 0 ? (
        <ul className="mt-4 space-y-1 text-xs text-muted-foreground" data-testid="personalization-overview-warnings">
          {warnings.map((warning) => (
            <li key={warning}>{t(`personalization.warning.${warning}`)}</li>
          ))}
        </ul>
      ) : null}

      <div className="mt-5 border-t border-border/70 pt-4">
        <h4 className="mb-3 text-sm font-semibold">{t("personalization.overview.agents.title")}</h4>
        {rowsQuery.isPending ? (
          <div className="flex items-center gap-2 text-sm text-muted-foreground">
            <LoaderCircle className="h-4 w-4 animate-spin" />
            {t("personalization.overview.agents.loading")}
          </div>
        ) : (
          <AgentOverviewList rows={rows} />
        )}
      </div>
    </SectionPanel>
  );
}
