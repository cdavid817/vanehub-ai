import { ArrowLeft, MessagesSquare, RefreshCw } from "lucide-react";
import { useCallback, useEffect, useState } from "react";
import { useTranslation } from "react-i18next";
import { Button } from "../../components/ui/button";
import type { ImConnectorHealth, ImConnectorKind, ImConnectorView, WeChatAuthorization } from "../../contracts/im";
import { imService } from "../../services/runtime-im-client";
import { detectRuntimeKind } from "../../services/runtime-adapter";
import { ImConnectorRow } from "./im/im-connector-row";
import { ImWeChatAuthorization } from "./im/im-wechat-authorization";
import { PageHeader } from "./page-parts";

type PendingByKind = Partial<Record<ImConnectorKind, string>>;

export function ImPage({ onReturn, searchTerm }: { onReturn?: () => void; searchTerm: string }) {
  const { t } = useTranslation();
  const isWebRuntime = detectRuntimeKind() !== "tauri";
  const [connectors, setConnectors] = useState<ImConnectorView[]>([]);
  const [pending, setPending] = useState<PendingByKind>({});
  const [authorization, setAuthorization] = useState<WeChatAuthorization | null>(null);
  const [authorizationPending, setAuthorizationPending] = useState(false);
  const [notice, setNotice] = useState<string | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [loading, setLoading] = useState(true);

  const load = useCallback(async () => {
    setError(null);
    setLoading(true);
    try {
      const [viewsResult] = await Promise.allSettled([imService.listConnectors()]);
      const loadErrors = [viewsResult]
        .filter((result): result is PromiseRejectedResult => result.status === "rejected")
        .map((result) => imErrorMessage(result.reason, t));
      if (viewsResult.status === "fulfilled") setConnectors(viewsResult.value);
      else setConnectors([]);
      setError(loadErrors[0] ?? null);
    } catch (reason) {
      setError(imErrorMessage(reason, t));
    } finally {
      setLoading(false);
    }
  }, [t]);

  useEffect(() => { void load(); }, [load]);
  useEffect(() => {
    let disposed = false;
    let unsubscribe: (() => void) | undefined;
    void imService.subscribeLifecycle((health) => {
      if (!disposed) setConnectors((current) => applyLifecycleUpdate(current, health));
    }).then((cleanup) => {
      if (disposed) cleanup();
      else unsubscribe = cleanup;
    }).catch(() => {
      // Manual refresh remains the recovery path when native event subscription is unavailable.
    });
    return () => {
      disposed = true;
      unsubscribe?.();
    };
  }, []);
  async function connectorAction(kind: ImConnectorKind, action: string, credentials?: Record<string, string>): Promise<boolean> {
    setPending((current) => ({ ...current, [kind]: action }));
    setError(null);
    setNotice(null);
    try {
      const view = connectors.find((item) => item.descriptor.kind === kind);
      if (!view) return false;
      if (action === "save") await imService.saveConnector({ kind, enabled: view.config.enabled, displayName: view.config.displayName, publicConfig: view.config.publicConfig, credentials });
      if (action === "enable" || action === "disable") await imService.setConnectorEnabled(kind, action === "enable");
      if (action === "test") await imService.testConnector(kind);
      if (action === "restart") await imService.restartConnector(kind);
      if (action === "clear") await imService.clearConnector(kind);
      setNotice(t(`im.notice.${action}`));
      setConnectors(await imService.listConnectors());
      return true;
    } catch (reason) {
      setError(imErrorMessage(reason, t));
      return false;
    } finally {
      setPending((current) => ({ ...current, [kind]: undefined }));
    }
  }

  async function authorizationAction(action: "begin" | "poll" | "cancel") {
    setAuthorizationPending(true);
    setError(null);
    try {
      if (action === "begin") setAuthorization(await imService.beginWeChatAuthorization());
      if (action === "poll") setAuthorization(await imService.pollWeChatAuthorization());
      if (action === "cancel") {
        await imService.cancelWeChatAuthorization();
        setAuthorization(null);
      }
      setConnectors(await imService.listConnectors());
    } catch (reason) {
      setError(imErrorMessage(reason, t));
    } finally {
      setAuthorizationPending(false);
    }
  }

  return (
    <div className="space-y-4">
      <PageHeader
        actions={<Button disabled={loading} onClick={() => void load()} variant="outline"><RefreshCw aria-hidden="true" />{t("im.actions.refresh")}</Button>}
        description={t("im.description")}
        icon={MessagesSquare}
        title={t("im.title")}
      />
      {isWebRuntime ? <div className="rounded-md border p-3 text-sm ucd-status-warning">{t("im.webNotice")}</div> : null}
      <div className="ucd-muted-panel flex flex-wrap items-center justify-between gap-3 rounded-lg p-3 text-sm">
        <p className="text-muted-foreground">{t("im.sessionBindingGuidance")}</p>
        {onReturn ? <Button onClick={onReturn} size="sm" variant="outline"><ArrowLeft aria-hidden="true" />{t("im.openSession")}</Button> : null}
      </div>
      {error ? <div aria-live="assertive" className="rounded-md border p-3 text-sm ucd-status-danger">{error}</div> : null}
      {notice ? <div aria-live="polite" className="rounded-md border p-3 text-sm ucd-status-success">{notice}</div> : null}
      <div className="space-y-3">
        {loading ? <div className="ucd-panel rounded-lg p-6 text-sm text-muted-foreground">{t("im.loading")}</div> : connectors.map((view) => (
          <ImConnectorRow
            authorization={view.descriptor.kind === "weixin" ? <ImWeChatAuthorization authorization={authorization} onBegin={() => void authorizationAction("begin")} onCancel={() => void authorizationAction("cancel")} onPoll={() => void authorizationAction("poll")} pending={authorizationPending} /> : undefined}
            key={view.descriptor.kind}
            onAction={(action, credentials) => connectorAction(view.descriptor.kind, action, credentials)}
            pendingAction={pending[view.descriptor.kind] ?? null}
            searchTerm={searchTerm}
            view={view}
          />
        ))}
      </div>
    </div>
  );
}

export function imErrorMessage(reason: unknown, t: ReturnType<typeof useTranslation>["t"]): string {
  const message = reason instanceof Error ? reason.message : String(reason);
  if (message.includes("communications-repository-failed")) return t("im.errors.repositoryFailed");
  if (message.includes("communications-repository-unavailable")) return t("im.errors.repositoryUnavailable");
  return message;
}

export function applyLifecycleUpdate(
  connectors: ImConnectorView[],
  health: ImConnectorHealth,
): ImConnectorView[] {
  return connectors.map((connector) => (
    connector.descriptor.kind === health.kind && health.generation >= connector.health.generation
      ? { ...connector, health }
      : connector
  ));
}
