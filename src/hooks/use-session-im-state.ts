import { useCallback, useEffect, useMemo, useRef, useState } from "react";
import type {
  ImConnectorHealth,
  ImConnectorKind,
  ImConnectorView,
  ImPairingStart,
  ImSessionBinding,
  ImSessionAccess,
} from "../contracts/im";
import type { ImService } from "../services/im-service";
import { imService as runtimeImService } from "../services/runtime-im-client";

function errorMessage(reason: unknown): string {
  return reason instanceof Error ? reason.message : String(reason);
}

export function useSessionImState(
  sessionId: string | null,
  service: ImService = runtimeImService,
  /**
   * Whether the pane reading this is on screen.
   *
   * The connector lifecycle subscription is the expensive half: it stays open for as long as the
   * effect lives, so a hidden pane holds a live channel for updates nobody can see. The state
   * itself survives — a reader returning to the tab sees what was last loaded while the reload
   * runs.
   */
  active = true,
) {
  const [connectors, setConnectors] = useState<ImConnectorView[]>([]);
  const [binding, setBinding] = useState<ImSessionBinding | null>(null);
  const [access, setAccessState] = useState<ImSessionAccess | null>(null);
  const [pairing, setPairingState] = useState<ImPairingStart | null>(null);
  const [selectedConnector, setSelectedConnectorState] = useState<ImConnectorKind>("feishu");
  const [pending, setPending] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const pairingRef = useRef<ImPairingStart | null>(null);
  const requestRef = useRef(0);
  const activeSessionRef = useRef(sessionId);
  const selectedConnectorRef = useRef<ImConnectorKind>("feishu");
  activeSessionRef.current = sessionId;

  const setPairing = useCallback((value: ImPairingStart | null) => {
    pairingRef.current = value;
    setPairingState(value);
  }, []);

  const reload = useCallback(async () => {
    const request = requestRef.current + 1;
    requestRef.current = request;
    if (!sessionId) {
      setBinding(null);
      setAccessState(null);
      setPairing(null);
      return;
    }
    setError(null);
    try {
      const views = await service.listConnectors();
      const ready = views.filter((view) => (
        view.config.enabled && view.health.lifecycle === "connected"
      ));
      let connector = selectedConnectorRef.current;
      let snapshot = await service.getSessionBinding(sessionId, connector);
      const preferred = snapshot.binding?.connector
        ?? (ready.some((view) => view.descriptor.kind === connector)
          ? connector
          : ready[0]?.descriptor.kind ?? connector);
      if (!snapshot.binding && preferred !== connector) {
        connector = preferred;
        snapshot = await service.getSessionBinding(sessionId, connector);
      } else {
        connector = preferred;
      }
      if (requestRef.current !== request) return;
      setConnectors(views);
      selectedConnectorRef.current = connector;
      setSelectedConnectorState(connector);
      setBinding(snapshot.binding);
      setAccessState(snapshot.access);
      if (snapshot.binding || !snapshot.pendingConnector) setPairing(null);
    } catch (reason) {
      if (requestRef.current === request) setError(errorMessage(reason));
    }
  }, [service, sessionId, setPairing]);

  useEffect(() => {
    if (!active) return;
    let disposed = false;
    let unsubscribe: (() => void) | null = null;
    setBinding(null);
    setAccessState(null);
    setPairing(null);
    setPending(false);
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
    // `active` belongs here: becoming visible has to reopen the subscription, and becoming hidden
    // has to run the cleanup that cancels a pending pairing.
  }, [active, reload, service, sessionId, setPairing]);

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

  const setAccess = useCallback(async (enabled: boolean) => {
    if (!sessionId) return null;
    setPending(true);
    setError(null);
    try {
      const connector = selectedConnectorRef.current;
      const next = await service.setSessionAccess(sessionId, connector, enabled);
      if (activeSessionRef.current !== sessionId) return null;
      setAccessState(next);
      return next;
    } catch (reason) {
      if (activeSessionRef.current === sessionId) setError(errorMessage(reason));
      return null;
    } finally {
      if (activeSessionRef.current === sessionId) setPending(false);
    }
  }, [service, sessionId]);

  const selectConnector = useCallback(async (connector: ImConnectorKind) => {
    if (!sessionId || binding || pairingRef.current) return;
    const request = requestRef.current + 1;
    requestRef.current = request;
    selectedConnectorRef.current = connector;
    setSelectedConnectorState(connector);
    setPending(true);
    setError(null);
    try {
      const snapshot = await service.getSessionBinding(sessionId, connector);
      if (requestRef.current !== request || activeSessionRef.current !== sessionId) return;
      setBinding(snapshot.binding);
      setAccessState(snapshot.access);
    } catch (reason) {
      if (requestRef.current === request) setError(errorMessage(reason));
    } finally {
      if (requestRef.current === request) setPending(false);
    }
  }, [binding, service, sessionId]);

  return {
    access,
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
    selectedConnector,
    selectConnector,
    setAccess,
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
