import type { ReactNode } from "react";
import type { McpServerConfig } from "../../../types/mcp";

/**
 * Task 12.18 audit finding: MCP's real grouping axis is config *scope* (user vs. project layers),
 * not a list/table row shape -- neither `EntityList` (a flat virtualized single-select list) nor
 * `DataTable` (a row/tabular model) has a grouped-section shape, so this stays a small bespoke
 * page-local component instead of forcing this page's real shape onto a primitive that does not
 * fit it, or inventing a new shared grouping primitive from a single consumer.
 */
export function McpScopeSection({
  renderCard,
  servers,
  title,
}: {
  renderCard: (server: McpServerConfig) => ReactNode;
  servers: McpServerConfig[];
  title: string;
}) {
  if (!servers.length) return null;
  return (
    <div className="space-y-3">
      <div className="flex items-center justify-between gap-2 border-b border-border/70 pb-2">
        <h3 className="text-sm font-semibold text-foreground">{title}</h3>
        <span className="text-xs text-muted-foreground">{servers.length}</span>
      </div>
      <div className="grid gap-4 lg:grid-cols-2 xl:grid-cols-3">{servers.map((server) => renderCard(server))}</div>
    </div>
  );
}
