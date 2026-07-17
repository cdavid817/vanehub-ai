import { ChevronDown, ExternalLink, RefreshCw, Save, TestTube2, Trash2 } from "lucide-react";
import { useMemo, useState } from "react";
import { useTranslation } from "react-i18next";
import { Badge } from "../../../components/ui/badge";
import { Button } from "../../../components/ui/button";
import type { ImConnectorView } from "../../../contracts/im";
import { cn } from "../../../lib/utils";
import { compactCredentials, connectorDocumentation, credentialFields, hasCompleteCredentials } from "./im-form";

interface ImConnectorRowProps {
  view: ImConnectorView;
  routingReady: boolean;
  searchTerm: string;
  pendingAction: string | null;
  onAction: (action: "save" | "enable" | "disable" | "test" | "restart" | "clear", credentials?: Record<string, string>) => Promise<void>;
  authorization?: React.ReactNode;
}

const statusTone = (lifecycle: ImConnectorView["health"]["lifecycle"]): "success" | "warning" | "danger" | "muted" => {
  if (lifecycle === "connected") return "success";
  if (lifecycle === "error" || lifecycle === "authorization-expired") return "danger";
  if (lifecycle === "connecting" || lifecycle === "reconnecting") return "warning";
  return "muted";
};

export function ImConnectorRow({ view, routingReady, searchTerm, pendingAction, onAction, authorization }: ImConnectorRowProps) {
  const { t } = useTranslation();
  const [expanded, setExpanded] = useState(false);
  const [credentials, setCredentials] = useState<Record<string, string>>({});
  const fields = view.descriptor.kind === "weixin" ? [] : credentialFields[view.descriptor.kind];
  const name = t(`im.platform.${view.descriptor.kind}.name`);
  const visible = useMemo(() => {
    const query = searchTerm.trim().toLowerCase();
    return !query || `${name} ${t(`im.platform.${view.descriptor.kind}.description`)}`.toLowerCase().includes(query);
  }, [name, searchTerm, t, view.descriptor.kind]);
  if (!visible) return null;

  const busy = pendingAction !== null;
  const canSave = view.descriptor.kind === "weixin" || view.hasCredentials || hasCompleteCredentials(view.descriptor.kind, credentials);
  async function save() {
    await onAction("save", compactCredentials(credentials));
    setCredentials({});
  }

  return (
    <section className="ucd-panel overflow-hidden rounded-lg" data-connector={view.descriptor.kind}>
      <div className="flex min-h-16 flex-wrap items-center gap-3 px-4 py-3">
        <button
          aria-expanded={expanded}
          className="flex min-w-0 flex-1 items-center gap-3 text-left outline-none focus-visible:ring-2 focus-visible:ring-ring"
          onClick={() => setExpanded((value) => !value)}
          type="button"
        >
          <span className="flex h-9 w-9 shrink-0 items-center justify-center rounded-md border border-border bg-[hsl(var(--panel-muted))] text-sm font-semibold text-primary">
            {name.slice(0, 2)}
          </span>
          <span className="min-w-0 flex-1">
            <span className="flex flex-wrap items-center gap-2">
              <span className="font-semibold">{name}</span>
              {view.descriptor.experimental ? <Badge tone="warning">{t("im.experimental")}</Badge> : null}
              <Badge tone={statusTone(view.health.lifecycle)}>{t(`im.status.${view.health.lifecycle}`)}</Badge>
            </span>
            <span className="mt-1 block truncate text-xs text-muted-foreground">{t(`im.platform.${view.descriptor.kind}.description`)}</span>
            <span className="mt-1 block text-xs text-muted-foreground">{t("im.status.updatedAt", { time: new Date(view.health.updatedAt).toLocaleTimeString() })}</span>
          </span>
          <ChevronDown className={cn("h-4 w-4 shrink-0 transition-transform", expanded && "rotate-180")} aria-hidden="true" />
        </button>
        <label className="flex h-9 items-center gap-2 text-xs font-medium">
          <input
            checked={view.config.enabled}
            className="h-4 w-4 accent-[hsl(var(--primary))]"
            disabled={busy || (!view.config.enabled && (!routingReady || !view.hasCredentials))}
            onChange={() => void onAction(view.config.enabled ? "disable" : "enable")}
            type="checkbox"
          />
          {view.config.enabled ? t("im.actions.enabled") : t("im.actions.disabled")}
        </label>
      </div>

      {expanded ? (
        <div className="border-t border-border px-4 py-4">
          {!routingReady ? <div className="mb-4 rounded-md border p-3 text-sm ucd-status-warning">{t("im.routing.incomplete")}</div> : null}
          {view.health.safeErrorCode ? <div className="mb-4 rounded-md border p-3 text-sm ucd-status-danger">{t("im.errors.safeCode", { code: view.health.safeErrorCode })}</div> : null}
          {fields.length > 0 ? (
            <div className="grid gap-3 md:grid-cols-2 xl:grid-cols-3">
              {fields.map((field) => (
                <label className="grid min-w-0 gap-1.5 text-sm" key={field.key}>
                  <span className="font-medium">{t(`im.fields.${field.key}`)}</span>
                  <input
                    autoComplete="off"
                    className="ucd-input h-9 min-w-0 rounded px-3 outline-none focus-visible:ring-2 focus-visible:ring-ring"
                    onChange={(event) => setCredentials((current) => ({ ...current, [field.key]: event.target.value }))}
                    placeholder={view.hasCredentials ? t("im.credentials.configured") : t("im.credentials.enter")}
                    type={field.secret ? "password" : "text"}
                    value={credentials[field.key] ?? ""}
                  />
                </label>
              ))}
            </div>
          ) : authorization}
          <div className="mt-4 flex flex-wrap items-center gap-2 border-t border-border/70 pt-3">
            {fields.length > 0 ? <Button disabled={busy || !canSave} onClick={() => void save()} size="sm"><Save aria-hidden="true" />{pendingAction === "save" ? t("im.actions.saving") : t("im.actions.save")}</Button> : null}
            <Button disabled={busy || !view.hasCredentials} onClick={() => void onAction("test")} size="sm" variant="outline"><TestTube2 aria-hidden="true" />{t("im.actions.test")}</Button>
            <Button disabled={busy || !view.config.enabled} onClick={() => void onAction("restart")} size="sm" variant="outline"><RefreshCw aria-hidden="true" />{t("im.actions.retry")}</Button>
            <Button asChild size="sm" variant="outline"><a href={connectorDocumentation[view.descriptor.kind]} rel="noreferrer" target="_blank"><ExternalLink aria-hidden="true" />{t("im.actions.documentation")}</a></Button>
            <Button className="sm:ml-auto" disabled={busy || (!view.hasCredentials && !view.config.enabled)} onClick={() => void onAction("clear")} size="sm" variant="ghost"><Trash2 aria-hidden="true" />{t("im.actions.clear")}</Button>
          </div>
          {pendingAction ? <p aria-live="polite" className="mt-3 text-xs text-muted-foreground">{t("im.actions.pending", { action: t(`im.actions.${pendingAction}`) })}</p> : null}
        </div>
      ) : null}
    </section>
  );
}
