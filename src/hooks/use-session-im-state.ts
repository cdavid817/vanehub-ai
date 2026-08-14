import { useCallback, useEffect, useMemo, useRef, useState } from "react";
import type {
  ImConnectorHealth,
  ImConnectorKind,
  ImConnectorView,
  ImPairingStart,
  ImSessionBinding,
} from "../contracts/im";
import type { ImService } from "../services/im-service";
import { imService as runtimeImService } from "../services/runtime-im-client";

function errorMessage(reason: unknown): string {
  return reason instanceof Error ? reason.message : String(reason);
}

export function useSessionImState(
  sessionId: string | null,
  service: ImService = runtimeImService,
) {
  const [connectors, setConnectors] = useState<ImConnectorView[]>([]);
  const [binding, setBinding] = useState<ImSessionBinding | null>(null);
  const [pairing, setPairingState] = useState<ImPairingStart | null>(null);
  const [pending, setPending] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const pairingRef = useRef<ImPairingStart | null>(null);
  const requestRef = useRef(0);

  const setPairing = useCallback((value: ImPairingStart | null) => {
    pairingRef.current = value;
    setPairingState(value);
  }, []);

  const reload = useCallback(async () => {
    const request = requestRef.current + 1;
    requestRef.current = request;
    if (!sessionId) {
      setBinding(null);
      setPairing(null);
      return;
    }
    setError(null);
    try {
      const [views, snapshot] = await Promise.all([
        service.listConnectors(),
        service.getSessionBinding(sessionId),
      ]);
      if (requestRef.current !== request) return;
      setConnectors(views);
      setBinding(snapshot.binding);
      if (snapshot.binding || !snapshot.pendingConnector) setPairing(null);
    } catch (reason) {
      if (requestRef.current === request) setError(errorMessage(reason));
    }
  }, [service, sessionId, setPairing]);

  useEffect(() => {
    let disposed = false;
    let unsubscribe: (() => void) | null = null;
    setBinding(null);
    setPairing(null);
    setError(null);
    void reload();
    void service.subscribeLifecycle((health: ImConnectorHealth) => {
      if (disposed) return;
      setConnectors((current) => current.map((connector) => (
        connector.descriptor.kind === health.kind ? { ...connector, health } : connector
      )));
    }).then((cleanup) => {
      if (disposed) cleanup();
      else unsubscribe = cleanup;
    }).catch((reason: unknown) => {
      if (!disposed) setError(errorMessage(reason));
    });
    return () => {
      disposed = true;
      requestRef.current += 1;
      unsubscribe?.();
      const activePairing = pairingRef.current;
      if (activePairing?.sessionId === sessionId) {
        pairingRef.current = null;
        void service.cancelPairing(activePairing.sessionId, activePairing.connector);
      }
    };
  }, [reload, service, sessionId, setPairing]);

  useEffect(() => {
    if (!pairing) return undefined;
    const expiresIn = Math.max(0, Date.parse(pairing.expiresAt) - Date.now());
    const expire = globalThis.setTimeout(() => {
      const activePairing = pairingRef.current;
      if (!activePairing) return;
      setPairing(null);
      setError("im-pairing-expired");
      void service.cancelPairing(activePairing.sessionId, activePairing.connector);
    }, expiresIn);
    const poll = globalThis.setInterval(() => { void reload(); }, 2_000);
    return () => {
      globalThis.clearTimeout(expire);
      globalThis.clearInterval(poll);
    };
  }, [pairing, reload, service, setPairing]);

  const mutate = useCallback(async (action: () => Promise<ImSessionBinding | boolean>) => {
    setPending(true);
    setError(null);
    try {
      const result = await action();
      if (typeof result === "boolean") {
        if (result) setBinding(null);
      } else {
        setBinding(result);
      }
      return result;
    } catch (reason) {
      setError(errorMessage(reason));
      return null;
    } finally {
      setPending(false);
    }
  }, []);

  const beginPairing = useCallback(async (connector: ImConnectorKind, replaceExisting: boolean) => {
    if (!sessionId) return null;
    setPending(true);
    setError(null);
    try {
      const next = await service.beginPairing(sessionId, connector, replaceExisting);
      setPairing(next);
      return next;
    } catch (reason) {
      setError(errorMessage(reason));
      return null;
    } finally {
      setPending(false);
    }
  }, [service, sessionId, setPairing]);

  const cancelPairing = useCallback(async () => {
    const activePairing = pairingRef.current;
    if (!activePairing) return false;
    setPending(true);
    setError(null);
    try {
      const removed = await service.cancelPairing(activePairing.sessionId, activePairing.connector);
      setPairing(null);
      return removed;
    } catch (reason) {
      setError(errorMessage(reason));
      return false;
    } finally {
      setPending(false);
    }
  }, [service, setPairing]);

  const retryPairing = useCallback(async () => {
    const activePairing = pairingRef.current;
    if (!activePairing) return null;
    setPending(true);
    setError(null);
    setPairing(null);
    try {
      await service.cancelPairing(activePairing.sessionId, activePairing.connector);
      const next = await service.beginPairing(
        activePairing.sessionId,
        activePairing.connector,
        activePairing.replaceExisting,
      );
      setPairing(next);
      return next;
    } catch (reason) {
      setError(errorMessage(reason));
      return null;
    } finally {
      setPending(false);
    }
  }, [service, setPairing]);

  return {
    beginPairing,
    binding,
    cancelPairing,
    connectors,
    error,
    pairing,
    pending,
    readyConnectors: useMemo(() => connectors.filter((connector) => (
      connector.config.enabled && connector.health.lifecycle === "connected"
    )), [connectors]),
    reload,
    retryPairing,
    removeBinding: () => sessionId
      ? mutate(() => service.removeSessionBinding(sessionId))
      : Promise.resolve(null),
    setNotifications: (enabled: boolean) => sessionId
      ? mutate(() => service.setCompletionNotifications(sessionId, enabled))
      : Promise.resolve(null),
    setPaused: (paused: boolean) => sessionId
      ? mutate(() => service.setBindingPaused(sessionId, paused))
      : Promise.resolve(null),
  };
}
