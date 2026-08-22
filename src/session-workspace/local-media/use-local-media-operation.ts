import { useCallback, useEffect, useRef, useState } from "react";

import type { LocalMediaService } from "../../services/local-media-service";
import type { LocalMediaErrorCode, LocalMediaOperationResult } from "../../types/local-media";
import { localMediaErrorCodeFrom } from "./local-media-errors";

/** Matches the repository's other operation pollers closely enough to feel the same. */
const POLL_INTERVAL_MS = 250;

export interface OperationWatch {
  /** Begin watching an operation. Replaces any previous watch on this hook instance. */
  watch: (operationId: string) => void;
  /** Stop watching without cancelling the native operation. */
  release: () => void;
  /** The operation currently being watched, or `null`. */
  operationId: string | null;
}

/**
 * Poll one local-media operation until it settles.
 *
 * Polling rather than subscribing because the native side exposes no event channel for these, and
 * the repository's other long-running work polls the same way. The interval is deliberately short:
 * the user is holding a button or staring at a spinner, and a slower poll shows up as lag between
 * the engine finishing and the draft changing.
 */
export function useLocalMediaOperation(
  service: LocalMediaService,
  onSettled: (
    outcome:
      | { status: "succeeded"; result: LocalMediaOperationResult }
      | { status: "failed"; code: LocalMediaErrorCode },
    operationId: string,
  ) => void,
): OperationWatch {
  const [operationId, setOperationId] = useState<string | null>(null);
  const settledRef = useRef<(typeof onSettled) | null>(null);
  settledRef.current = onSettled;

  useEffect(() => {
    if (!operationId) return undefined;
    let cancelled = false;

    const poll = async () => {
      try {
        const result = await service.getOperationResult(operationId);
        if (cancelled || !result) return;
        setOperationId(null);
        settledRef.current?.({ status: "succeeded", result }, operationId);
      } catch (error) {
        if (cancelled) return;
        setOperationId(null);
        settledRef.current?.(
          { status: "failed", code: localMediaErrorCodeFrom(error) },
          operationId,
        );
      }
    };

    void poll();
    const timer = window.setInterval(() => void poll(), POLL_INTERVAL_MS);
    return () => {
      // Unmount stops the poll but not the operation: the native side owns cleanup, and a result
      // that arrives after the composer is gone is simply never read.
      cancelled = true;
      window.clearInterval(timer);
    };
  }, [operationId, service]);

  const watch = useCallback((next: string) => setOperationId(next), []);
  const release = useCallback(() => setOperationId(null), []);

  return { watch, release, operationId };
}
