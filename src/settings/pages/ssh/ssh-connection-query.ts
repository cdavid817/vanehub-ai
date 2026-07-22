import type { QueryClient } from "@tanstack/react-query";

export const sshConnectionsQueryKey = ["ssh-connections"] as const;

export function refreshSshConnections(queryClient: QueryClient) {
  return queryClient.invalidateQueries({ queryKey: sshConnectionsQueryKey });
}
