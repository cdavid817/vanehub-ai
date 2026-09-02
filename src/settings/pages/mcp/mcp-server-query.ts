import type { QueryClient } from "@tanstack/react-query";
import { mcpService } from "../../../services/runtime-mcp-client";
import type { McpServerConfig, McpServerStatus } from "../../../types/mcp";

export type McpStatusMap = Record<string, McpServerStatus>;

export interface McpServersAndStatuses {
  servers: McpServerConfig[];
  statuses: McpStatusMap;
}

export const mcpServersQueryKey = ["mcp", "servers"] as const;

/** Task 12.18 extraction: this loader plus its query key used to live inline in `mcp-page.tsx`,
 *  matching `ssh-connection-query.ts`'s own precedent of giving a page's query concerns their own
 *  file once the page itself is over the line budget. */
export async function loadMcpServersAndStatuses(): Promise<McpServersAndStatuses> {
  const servers = await mcpService.listServers();
  const entries = await Promise.all(
    servers.map(async (server) => [server.name, await mcpService.getServerStatus(server.name)] as const),
  );

  return {
    servers,
    statuses: Object.fromEntries(entries) as McpStatusMap,
  };
}

export function refreshMcpServers(queryClient: QueryClient) {
  return queryClient.invalidateQueries({ queryKey: mcpServersQueryKey });
}
