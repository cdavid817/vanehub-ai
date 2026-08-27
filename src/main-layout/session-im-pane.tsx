import { Bell, Link2, Link2Off, Pause, Play, RefreshCw } from "lucide-react";
import { useState } from "react";
import { useTranslation } from "react-i18next";
import { Button } from "../components/ui/button";
import { formatAppDateTime } from "../i18n/format";
import { useSessionImState } from "../hooks/use-session-im-state";
import type { ImService } from "../services/im-service";
import { SessionImAccessToggle } from "./session-im-access-toggle";

export function SessionImPane({
  onOpenSettings,
  service,
  sessionId,
}: {
  onOpenSettings?: () => void;
  service?: ImService;
  sessionId: string | null;
}) {
  const { i18n, t } = useTranslation();
  const [replaceExisting, setReplaceExisting] = useState(false);
  const [confirmRemoval, setConfirmRemoval] = useState(false);
  const state = useSessionImState(sessionId, service);
  const { access, binding, connectors, error, pairing, pending } = state;

  if (!sessionId) return <Empty>{t("im.session.noSession")}</Empty>;
  const boundConnector = binding
    ? connectors.find((connector) => connector.descriptor.kind === binding.connector)
    : undefined;
  const selectedConnector = connectors.find(
    (connector) => connector.descriptor.kind === state.selectedConnector,
  );
  const selectedReady = state.readyConnectors.some(
    (connector) => connector.descriptor.kind === state.selectedConnector,
  );
  const selectableConnectors = binding && boundConnector
    ? [boundConnector]
    : state.readyConnectors.length
      ? state.readyConnectors
      : selectedConnector ? [selectedConnector] : [];
  return (
    <div className="grid gap-3" data-testid="session-im-pane">
      {error ? <div className="rounded-md border p-2 text-xs ucd-status-danger" role="alert">{error}</div> : null}
      <label className="grid gap-1 text-xs font-medium">
        <span>{t("im.session.connector")}</span>
        <select
          aria-label={t("im.session.connector")}
          className="ucd-input rounded-md px-2 py-1.5"
          disabled={Boolean(binding || pairing || pending) || selectableConnectors.length === 0}
          onChange={(event) => void state.selectConnector(event.target.value as typeof state.selectedConnector)}
          value={state.selectedConnector}
        >
          {selectableConnectors.length === 0 ? (
            <option value={state.selectedConnector}>{t(`im.platform.${state.selectedConnector}.name`)}</option>
          ) : selectableConnectors.map((connector) => (
            <option key={connector.descriptor.kind} value={connector.descriptor.kind}>
              {t(`im.platform.${connector.descriptor.kind}.name`)}
            </option>
          ))}
        </select>
      </label>
      <SessionImAccessToggle
        binding={binding}
        enabled={access?.enabled ?? false}
        onChange={state.setAccess}
        pending={pending}
        platformName={t(`im.platform.${state.selectedConnector}.name`)}
      />
      {!access?.enabled ? (
        <p className="px-1 text-xs text-muted-foreground">{t("im.session.access.optedOut")}</p>
      ) : binding ? (
        <section className="ucd-muted-panel grid gap-3 rounded-lg p-3">
          <div className="flex items-start justify-between gap-2">
            <div>
              <h3 className="text-sm font-semibold">{t(`im.platform.${binding.connector}.name`)}</h3>
              <p className="mt-1 text-xs text-muted-foreground">
                {t(`im.session.status.${binding.state}`)} · {boundConnector
                  ? t(`im.status.${boundConnector.health.lifecycle}`)
                  : t("im.status.unconfigured")}
              </p>
              <p className="mt-1 text-xs text-muted-foreground">
                {formatAppDateTime(binding.updatedAt, i18n.language, {
                  dateStyle: "medium",
                  timeStyle: "short",
                })}
              </p>
            </div>
            <Link2 aria-hidden="true" className="h-4 w-4 text-primary" />
          </div>
          <label className="flex items-center justify-between gap-3 text-xs">
            <span className="flex items-center gap-2"><Bell className="h-3.5 w-3.5" />{t("im.session.notifications")}</span>
            <input
              checked={binding.completionNotifications}
              disabled={pending}
              onChange={(event) => void state.setNotifications(event.target.checked)}
              type="checkbox"
            />
          </label>
          <div className="grid grid-cols-2 gap-2">
            <Button
              disabled={pending}
              onClick={() => void state.setPaused(binding.state === "active")}
              size="sm"
              variant="outline"
            >
              {binding.state === "active" ? <Pause /> : <Play />}
              {t(binding.state === "active" ? "im.session.pause" : "im.session.resume")}
            </Button>
            <Button disabled={pending} onClick={() => setConfirmRemoval(true)} size="sm" variant="outline">
              <Link2Off />{t("im.session.remove")}
            </Button>
          </div>
          {confirmRemoval ? (
            <div aria-live="polite" className="grid gap-2 rounded-md border border-destructive/40 p-2 text-xs">
              <p>{t("im.session.removeConfirm")}</p>
              <div className="grid grid-cols-2 gap-2">
                <Button onClick={() => setConfirmRemoval(false)} size="sm" variant="ghost">{t("im.session.keep")}</Button>
                <Button
                  className="bg-destructive text-destructive-foreground hover:bg-destructive/90"
                  disabled={pending}
                  onClick={() => void state.removeBinding().then(() => setConfirmRemoval(false))}
                  size="sm"
                  variant="default"
                >{t("im.session.confirmRemove")}</Button>
              </div>
            </div>
          ) : null}
        </section>
      ) : pairing ? (
        <section aria-live="polite" className="ucd-muted-panel grid gap-3 rounded-lg p-3">
          <div>
            <h3 className="text-sm font-semibold">{t("im.session.pairingTitle")}</h3>
            <p className="mt-1 text-xs text-muted-foreground">{t("im.session.pairingHint")}</p>
          </div>
          <code className="select-all rounded-md border border-primary/30 bg-background p-3 text-center text-base font-semibold tracking-wider">
            /bind {pairing.code}
          </code>
          <p className="text-xs text-muted-foreground">
            {t("im.session.expiresAt", {
              time: formatAppDateTime(pairing.expiresAt, i18n.language, {
                hour: "2-digit",
                minute: "2-digit",
              }),
            })}
          </p>
          <div className="grid grid-cols-2 gap-2">
            <Button
              disabled={pending}
              onClick={() => void state.retryPairing()}
              size="sm"
              variant="outline"
            ><RefreshCw />{t("im.session.retry")}</Button>
            <Button
              disabled={pending}
              onClick={() => void state.cancelPairing()}
              size="sm"
              variant="outline"
            >{t("im.session.cancel")}</Button>
          </div>
        </section>
      ) : (
        <section className="ucd-muted-panel grid gap-3 rounded-lg p-3">
          <div>
            <h3 className="text-sm font-semibold">{t("im.session.connectTitle")}</h3>
            <p className="mt-1 text-xs text-muted-foreground">{t("im.session.connectDescription")}</p>
          </div>
          {state.readyConnectors.length ? (
            <Button disabled={pending || !selectedReady} onClick={() => void state.beginPairing(state.selectedConnector, replaceExisting)} variant="outline">
              <Link2 />{t(`im.platform.${state.selectedConnector}.name`)}
            </Button>
          ) : (
            <div className="grid gap-2 text-xs text-muted-foreground">
              <p>{t("im.session.noConnector")}</p>
              {onOpenSettings ? <Button onClick={onOpenSettings} size="sm" variant="outline">{t("im.session.openSettings")}</Button> : null}
            </div>
          )}
          <label className="flex items-center gap-2 text-xs text-muted-foreground">
            <input checked={replaceExisting} onChange={(event) => setReplaceExisting(event.target.checked)} type="checkbox" />
            {t("im.session.replaceExisting")}
          </label>
          <Button disabled={pending} onClick={() => void state.reload()} size="sm" variant="ghost"><RefreshCw />{t("im.actions.refresh")}</Button>
        </section>
      )}
    </div>
  );
}

function Empty({ children }: { children: string }) {
  return <p className="p-3 text-center text-xs text-muted-foreground">{children}</p>;
}
