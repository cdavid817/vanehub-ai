import { useEffect, useState } from "react";
import { agentService } from "../services/runtime-agent-client";
import { operationService } from "../services/runtime-operation-client";
import type { ReviewAction } from "../types/code-review";
import type { OperationTask } from "../types/operation";

export function useReviewAction(reviewId: string) {
  const [operation, setOperation] = useState<OperationTask | null>(null);
  const [error, setError] = useState<string | null>(null);
  useEffect(() => {
    if (!operation || ["succeeded", "failed", "cancelled"].includes(operation.status)) return;
    const timer = window.setInterval(() => {
      operationService.getOperationStatus(operation.id)
        .then(setOperation)
        .catch((reason: unknown) => setError(reason instanceof Error ? reason.message : String(reason)));
    }, 250);
    return () => window.clearInterval(timer);
  }, [operation]);
  return {
    operation,
    error,
    async start(action: ReviewAction) {
      setError(null);
      const started = await agentService.startCodeReviewAction(reviewId, action);
      setOperation(await operationService.getOperationStatus(started.operationId));
    },
    async cancel() {
      if (operation) setOperation(await operationService.cancelOperation(operation.id));
    },
  };
}
