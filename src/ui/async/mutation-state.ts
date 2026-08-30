import { useCallback, useState } from "react";
import type { DisplayableError } from "./async-view-state";

export interface MutationState {
  targetKey: string;
  operationId?: string;
  pending: boolean;
  error?: DisplayableError;
}

export interface MutationRegistryApi {
  registry: ReadonlyMap<string, MutationState>;
  get(targetKey: string): MutationState | undefined;
  begin(targetKey: string, operationId?: string): void;
  succeed(targetKey: string): void;
  fail(targetKey: string, error: DisplayableError): void;
  clear(targetKey: string): void;
}

/**
 * Per-target mutation tracking so a list row can show its own pending/error state without a
 * page-wide `isBusy` flag (design.md Decision 11: "mutation 只禁用目标动作，保留其他内容").
 * This registry does not hold domain data, so it cannot revert an optimistic update itself —
 * `fail()` is the signal for the caller to roll back whatever it optimistically applied.
 */
export function useMutationRegistry(): MutationRegistryApi {
  const [registry, setRegistry] = useState<ReadonlyMap<string, MutationState>>(() => new Map());

  const begin = useCallback((targetKey: string, operationId?: string) => {
    setRegistry((current) => {
      const next = new Map(current);
      next.set(targetKey, { targetKey, operationId, pending: true });
      return next;
    });
  }, []);

  const succeed = useCallback((targetKey: string) => {
    setRegistry((current) => {
      if (!current.has(targetKey)) return current;
      const next = new Map(current);
      next.delete(targetKey);
      return next;
    });
  }, []);

  const fail = useCallback((targetKey: string, error: DisplayableError) => {
    setRegistry((current) => {
      const next = new Map(current);
      const existing = next.get(targetKey);
      next.set(targetKey, { targetKey, operationId: existing?.operationId, pending: false, error });
      return next;
    });
  }, []);

  const clear = useCallback((targetKey: string) => {
    setRegistry((current) => {
      if (!current.has(targetKey)) return current;
      const next = new Map(current);
      next.delete(targetKey);
      return next;
    });
  }, []);

  const get = useCallback((targetKey: string) => registry.get(targetKey), [registry]);

  return { registry, get, begin, succeed, fail, clear };
}
