import { Bell, Link2, Link2Off, Pause, Play, RefreshCw } from "lucide-react";
import { useState } from "react";
import { useTranslation } from "react-i18next";
import { Button } from "../components/ui/button";
import { formatAppDateTime } from "../i18n/format";
import { useSessionImState } from "../hooks/use-session-im-state";
import type { ImService } from "../services/im-service";

export function SessionImPane({
  active = true,
  onOpenSettings,
  service,
  sessionId,
}: {
  /**
   * Whether this pane is the one on screen.
   *
   * Mounted either way — these panes hold local form state, and a reader who typed something,
   * checked another tab, and came back must find it still there. What stops is the reading: a
   * hidden pane polling its own service costs a request per pane per session open, for answers
   * nobody is looking at.
   *
   * Mutations are unaffected. React Query runs one to completion regardless of this flag, so a
   * write that was in flight when the reader switched away still finishes and still invalidates.
   */
  active?: boolean;
  onOpenSettings?: () => void;
  service?: ImService;
  sessionId: string | null;
}) {
  const { i18n, t } = useTranslation();
  const [replaceExisting, setReplaceExisting] = useState(false);
  const [confirmRemoval, setConfirmRemoval] = useState(false);
  const state = useSessionImState(sessionId, service, active);
  const { binding, connectors, error, pairing, pending, readyConnectors: ready } = state;

  if (!sessionId) return <Empty>{t("im.session.noSession")}</Empty>;
  const boundConnector = binding
    ? connectors.find((connector) => connector.descriptor.kind === binding.connector)
    : undefined;
  return (
    <div className="grid gap-3" data-testid="session-im-pane">
      {error ? <div className="rounded-md border p-2 text-xs ucd-status-danger" role="alert">{error}</div> : null}
      {binding ? (
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
          {ready.length ? ready.map((connector) => (
            <Button disabled={pending} key={connector.descriptor.kind} onClick={() => void state.beginPairing(connector.descriptor.kind, replaceExisting)} variant="outline">
              <Link2 />{t(`im.platform.${connector.descriptor.kind}.name`)}
            </Button>
          )) : (
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
