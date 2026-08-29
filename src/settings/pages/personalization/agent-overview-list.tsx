import { useTranslation } from "react-i18next";
import { Badge } from "../../../components/ui/badge";
import type { AgentOverviewRow, ControlState } from "./overview-model";

function ControlBadge({ state }: { state: ControlState }) {
  const { t } = useTranslation();
  if (state.kind === "unavailable") {
    // Named, not blanked: "this Agent cannot do it" is a different answer from "it is switched
    // off", and only one of them is worth the user opening the policy editor.
    return <Badge tone="muted">{t("personalization.overview.state.unavailableRuntime")}</Badge>;
  }
  return (
    <Badge tone={state.kind === "on" ? "success" : "muted"}>
      {t(`personalization.overview.state.${state.kind}`)}
    </Badge>
  );
}

function SourceLabel({ row }: { row: AgentOverviewRow }) {
  const { t } = useTranslation();
  if (row.sources.length === 0) {
    return <span className="text-muted-foreground">{t("personalization.overview.source.none")}</span>;
  }
  return (
    <span className="wrap-break-word">
      {row.sources
        .map((source) =>
          source.scopeKey
            ? `${t(`personalization.overview.source.${source.scopeKind}`)} (${source.scopeKey})`
            : t(`personalization.overview.source.${source.scopeKind}`),
        )
        .join(" + ")}
    </span>
  );
}

/**
 * One row per registered Agent, built entirely from what the registry reported.
 *
 * Nothing here knows which Agents exist. An Agent registered after this shipped gets a row with
 * its own capabilities, which is the whole point of resolving through the native preview instead
 * of reimplementing precedence here.
 */
export function AgentOverviewList({ rows }: { rows: readonly AgentOverviewRow[] }) {
  const { t } = useTranslation();

  if (rows.length === 0) {
    return (
      <p className="text-sm text-muted-foreground" data-testid="personalization-overview-no-agents">
        {t("personalization.overview.agents.empty")}
      </p>
    );
  }

  return (
    <div className="overflow-x-auto">
      <table className="w-full min-w-[640px] text-left text-sm">
        <thead className="text-xs uppercase tracking-wide text-muted-foreground">
          <tr>
            <th className="py-2 pr-4 font-medium">{t("personalization.overview.columns.agent")}</th>
            <th className="py-2 pr-4 font-medium">{t("personalization.overview.columns.instructions")}</th>
            <th className="py-2 pr-4 font-medium">{t("personalization.overview.columns.source")}</th>
            <th className="py-2 pr-4 font-medium">{t("personalization.overview.columns.memory")}</th>
            <th className="py-2 font-medium">{t("personalization.overview.columns.extraction")}</th>
          </tr>
        </thead>
        <tbody>
          {rows.map((row) => (
            <tr className="border-t border-border/70 align-top" data-testid={`personalization-overview-agent-${row.agentId}`} key={row.agentId}>
              <td className="py-3 pr-4">
                <div className="font-medium">{row.displayName}</div>
                <div className="text-xs text-muted-foreground">{row.agentId}</div>
              </td>
              <td className="py-3 pr-4">
                <ControlBadge state={row.instructions} />
                {row.instructions.kind === "on" ? (
                  <div className="mt-1 text-xs text-muted-foreground">
                    {t("personalization.overview.characters", { count: row.characters })}
                  </div>
                ) : null}
              </td>
              <td className="py-3 pr-4 text-xs">
                <SourceLabel row={row} />
              </td>
              <td className="py-3 pr-4">
                <ControlBadge state={row.memoryRead} />
                <div className="mt-1 text-xs text-muted-foreground" data-testid={`personalization-overview-delivery-${row.agentId}`}>
                  {t(`personalization.overview.delivery.${row.delivery}`)}
                </div>
              </td>
              <td className="py-3">
                <ControlBadge state={row.extraction} />
              </td>
            </tr>
          ))}
        </tbody>
      </table>
    </div>
  );
}
