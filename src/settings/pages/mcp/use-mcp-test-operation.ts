import { useEffect, useState } from "react";
import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import type { TFunction } from "i18next";
import { mcpService } from "../../../services/runtime-mcp-client";
import { operationService } from "../../../services/runtime-operation-client";
import type { McpServerConfig, McpTestResult } from "../../../types/mcp";
import type { OperationTask } from "../../../types/operation";
import type { MutationState } from "../../../ui/async/mutation-state";
import { formatMcpFailure, mcpMutationErrorMessage } from "./mcp-presentation";
import { mcpServersQueryKey } from "./mcp-server-query";

function mcpTestResult(result: OperationTask["result"]): McpTestResult | null {
  if (!result || typeof result !== "object") return null;
  if (typeof result.success !== "boolean") return null;
  if (!Array.isArray(result.tools)) return null;
  return result as unknown as McpTestResult;
}

/**
 * Task 12.18 extraction: "Test connection" is a two-phase mutation+polled-operation lifecycle --
 * the same shape Extensions' own operationMutation/activeOperation pair already established --
 * kept in its own hook because the page has 4 other independent mutations of its own, and this
 * one (plus its polling effect) was the single largest, most self-contained thing actually
 * inflating `mcp-page.tsx` past the line budget.
 */
export function useMcpTestOperation({
  onTestPassed,
  t,
}: {
  onTestPassed: (message: string) => void;
  t: TFunction;
}) {
  const queryClient = useQueryClient();
  const [activeOperationId, setActiveOperationId] = useState<string | null>(null);
  const [handledOperationId, setHandledOperationId] = useState<string | null>(null);

  const testMutation = useMutation({
    mutationFn: async (server: McpServerConfig) => ({
      operation: await mcpService.testConnection(server.name),
      server,
    }),
    onSuccess: ({ operation }) => {
      setActiveOperationId(operation.id);
      setHandledOperationId(null);
    },
  });

  const operationQuery = useQuery({
    enabled: activeOperationId !== null,
    queryFn: () => operationService.getOperationStatus(activeOperationId ?? ""),
    queryKey: ["operation", activeOperationId],
    refetchInterval: (query) =>
      query.state.data?.status === "queued" || query.state.data?.status === "running" ? 600 : false,
  });

  useEffect(() => {
    const operation = operationQuery.data;
    if (!operation || operation.id === handledOperationId) return;
    if (operation.status === "queued" || operation.status === "running") return;
    setHandledOperationId(operation.id);
    const result = mcpTestResult(operation.result);
    const name = operation.relatedEntityId ?? testMutation.variables?.name ?? "";
    if (operation.status !== "failed" && result?.success) {
      onTestPassed(t("mcp.notice.testPassed", { count: result.tools.length, name }));
    }
    void queryClient.invalidateQueries({ queryKey: mcpServersQueryKey });
  }, [handledOperationId, onTestPassed, operationQuery.data, queryClient, t, testMutation.variables?.name]);

  async function testServer(server: McpServerConfig) {
    await testMutation.mutateAsync(server).catch(() => undefined);
  }

  function stateFor(serverName: string): MutationState | undefined {
    if (testMutation.variables?.name === serverName) {
      if (testMutation.isPending) return { pending: true, targetKey: serverName };
      if (testMutation.isError) {
        return {
          error: { kind: "error", message: mcpMutationErrorMessage(t, testMutation.error), retryable: true },
          pending: false,
          targetKey: serverName,
        };
      }
    }
    const operation = operationQuery.data;
    if (operation?.relatedEntityId === serverName) {
      if (operation.status === "queued" || operation.status === "running") {
        return { operationId: operation.id, pending: true, targetKey: serverName };
      }
      const result = mcpTestResult(operation.result);
      if (operation.status === "failed" || result?.success === false) {
        return {
          error: {
            kind: "error",
            message: formatMcpFailure(t, result?.errorCode, operation.error ?? result?.error),
            retryable: true,
          },
          operationId: operation.id,
          pending: false,
          targetKey: serverName,
        };
      }
    }
    return undefined;
  }

  return { stateFor, testServer };
}
