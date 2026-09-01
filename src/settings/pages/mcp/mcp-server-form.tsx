import { Save } from "lucide-react";
import { useMemo, useState } from "react";
import { useTranslation } from "react-i18next";
import { ApplicationDialog } from "../../../components/ui/application-dialog";
import { Button } from "../../../components/ui/button";
import type { McpScope, McpServerConfig, McpTransportType } from "../../../types/mcp";
import { formatMcpFailure, mcpErrorFromUnknown, mcpTransportTranslationKey } from "./mcp-presentation";
import { type McpServerFormErrors, validateMcpServerForm } from "./mcp-server-validation";

function jsonText(value: Record<string, string> | null | undefined) {
  return JSON.stringify(value ?? {}, null, 2);
}

function hasEntries(value: Record<string, string> | null | undefined) {
  return Boolean(value && Object.keys(value).length > 0);
}

export function McpServerForm({
  server,
  onCancel,
  onSave,
}: {
  server?: McpServerConfig | null;
  onCancel: () => void;
  onSave: (server: McpServerConfig) => Promise<void>;
}) {
  const { t } = useTranslation();
  const editingName = server?.name ?? null;
  const [name, setName] = useState(server?.name ?? "");
  const [transportType, setTransportType] = useState<McpTransportType>(server?.transportType ?? "stdio");
  const [scope, setScope] = useState<McpScope>(server?.scope ?? "user");
  const [command, setCommand] = useState(server?.command ?? "");
  const [args, setArgs] = useState((server?.args ?? []).join("\n"));
  const [env, setEnv] = useState(jsonText(server?.env));
  // Env/headers can carry real credentials (a bearer token header, an API-key env var) already
  // saved for this server -- spec.md "Render a credential field" says avoid displaying a secret
  // by default, so an existing server's non-empty values start masked behind an explicit reveal.
  // A brand-new server has nothing saved to hide, so there is nothing to gate.
  const [envRevealed, setEnvRevealed] = useState(!hasEntries(server?.env));
  const [url, setUrl] = useState(server?.url ?? "");
  const [headers, setHeaders] = useState(jsonText(server?.headers));
  const [headersRevealed, setHeadersRevealed] = useState(!hasEntries(server?.headers));
  const [description, setDescription] = useState(server?.description ?? "");
  const [active, setActive] = useState(server?.active ?? true);
  const [error, setError] = useState<string | null>(null);
  const [fieldErrors, setFieldErrors] = useState<McpServerFormErrors>({});
  const [saving, setSaving] = useState(false);

  const title = useMemo(() => (editingName ? t("mcp.form.editTitle") : t("mcp.form.addTitle")), [editingName, t]);

  async function handleSubmit() {
    setError(null);
    setFieldErrors({});
    const result = validateMcpServerForm(
      {
        name,
        transportType,
        scope,
        command,
        args,
        env,
        url,
        headers,
        description,
        active,
      },
      {
        name: t("mcp.validation.name"),
        commandRequired: t("mcp.validation.commandRequired"),
        urlRequired: t("mcp.validation.urlRequired"),
        jsonObject: t("mcp.validation.jsonObject"),
        argsArray: t("mcp.validation.argsArray"),
      },
    );

    if (!result.success) {
      setFieldErrors(result.errors);
      setError(result.errors.form ?? t("mcp.form.fixErrors"));
      return;
    }

    setSaving(true);
    try {
      await onSave(result.config);
    } catch (err) {
      const failure = mcpErrorFromUnknown(err);
      setError(formatMcpFailure(t, failure.errorCode, failure.message));
    } finally {
      setSaving(false);
    }
  }

  return (
    <ApplicationDialog closeDisabled={saving} onClose={onCancel} title={title}>
        <div className="grid gap-3 text-sm md:grid-cols-2">
          <label className="grid gap-1">
            <span className="text-xs text-muted-foreground">{t("mcp.form.name")}</span>
            <input data-dialog-autofocus className="ucd-input h-9 rounded px-3 outline-hidden focus-visible:ring-2 focus-visible:ring-ring" value={name} onChange={(event) => setName(event.target.value)} />
            {fieldErrors.name ? <span className="text-xs text-[hsl(var(--danger))]">{fieldErrors.name}</span> : null}
          </label>
          <label className="grid gap-1">
            <span className="text-xs text-muted-foreground">{t("mcp.form.scope")}</span>
            <select className="ucd-input h-9 rounded px-3 outline-hidden focus-visible:ring-2 focus-visible:ring-ring" value={scope} onChange={(event) => setScope(event.target.value as McpScope)}>
              <option value="user">{t("mcp.scope.userConfig")}</option>
              <option value="project">{t("mcp.scope.projectConfig")}</option>
            </select>
          </label>
          <label className="grid gap-1">
            <span className="text-xs text-muted-foreground">{t("mcp.form.transport")}</span>
            <select className="ucd-input h-9 rounded px-3 outline-hidden focus-visible:ring-2 focus-visible:ring-ring" value={transportType} onChange={(event) => setTransportType(event.target.value as McpTransportType)}>
              <option value="stdio">{t(mcpTransportTranslationKey("stdio"))}</option>
              <option value="sse">{t(mcpTransportTranslationKey("sse"))}</option>
              <option value="streamable_http">{t(mcpTransportTranslationKey("streamable_http"))}</option>
            </select>
          </label>
          <label className="flex items-center gap-2 pt-5 text-sm">
            <input checked={active} onChange={(event) => setActive(event.target.checked)} type="checkbox" />
            {t("mcp.form.enabled")}
          </label>
        </div>

        <label className="mt-3 grid gap-1 text-sm">
          <span className="text-xs text-muted-foreground">{t("mcp.form.description")}</span>
          <input className="ucd-input h-9 rounded px-3 outline-hidden focus-visible:ring-2 focus-visible:ring-ring" value={description} onChange={(event) => setDescription(event.target.value)} />
        </label>

        {transportType === "stdio" ? (
          <div className="mt-3 grid gap-3">
            <label className="grid gap-1 text-sm">
              <span className="text-xs text-muted-foreground">{t("mcp.form.command")}</span>
              <input className="ucd-input h-9 rounded px-3 outline-hidden focus-visible:ring-2 focus-visible:ring-ring" value={command} onChange={(event) => setCommand(event.target.value)} />
              {fieldErrors.command ? <span className="text-xs text-[hsl(var(--danger))]">{fieldErrors.command}</span> : null}
            </label>
            <label className="grid gap-1 text-sm">
              <span className="text-xs text-muted-foreground">{t("mcp.form.args")}</span>
              <textarea className="ucd-input min-h-24 rounded p-3 font-mono text-xs outline-hidden focus-visible:ring-2 focus-visible:ring-ring" value={args} onChange={(event) => setArgs(event.target.value)} />
              {fieldErrors.args ? <span className="text-xs text-[hsl(var(--danger))]">{fieldErrors.args}</span> : null}
            </label>
            {/* A plain div, not `label`, when masked: implicit label association would fold the
                placeholder text and the Reveal button's own text together into one unusable
                accessible name for the button, since a label associates with every control it
                wraps, not just the one it visually sits above. */}
            <div className="grid gap-1 text-sm">
              <span className="text-xs text-muted-foreground">{t("mcp.form.envJson")}</span>
              {envRevealed ? (
                <textarea aria-label={t("mcp.form.envJson")} className="ucd-input min-h-28 rounded p-3 font-mono text-xs outline-hidden focus-visible:ring-2 focus-visible:ring-ring" value={env} onChange={(event) => setEnv(event.target.value)} />
              ) : (
                <div className="ucd-input flex min-h-28 flex-col items-start justify-center gap-2 rounded p-3 text-xs text-muted-foreground">
                  <span>{t("mcp.form.credentialsHidden")}</span>
                  <Button onClick={() => setEnvRevealed(true)} size="sm" type="button" variant="outline">{t("mcp.form.reveal")}</Button>
                </div>
              )}
              {fieldErrors.env ? <span className="text-xs text-[hsl(var(--danger))]">{fieldErrors.env}</span> : null}
            </div>
          </div>
        ) : (
          <div className="mt-3 grid gap-3">
            <label className="grid gap-1 text-sm">
              <span className="text-xs text-muted-foreground">{t("mcp.form.url")}</span>
              <input className="ucd-input h-9 rounded px-3 outline-hidden focus-visible:ring-2 focus-visible:ring-ring" value={url} onChange={(event) => setUrl(event.target.value)} />
              {fieldErrors.url ? <span className="text-xs text-[hsl(var(--danger))]">{fieldErrors.url}</span> : null}
            </label>
            <div className="grid gap-1 text-sm">
              <span className="text-xs text-muted-foreground">{t("mcp.form.headersJson")}</span>
              {headersRevealed ? (
                <textarea aria-label={t("mcp.form.headersJson")} className="ucd-input min-h-28 rounded p-3 font-mono text-xs outline-hidden focus-visible:ring-2 focus-visible:ring-ring" value={headers} onChange={(event) => setHeaders(event.target.value)} />
              ) : (
                <div className="ucd-input flex min-h-28 flex-col items-start justify-center gap-2 rounded p-3 text-xs text-muted-foreground">
                  <span>{t("mcp.form.credentialsHidden")}</span>
                  <Button onClick={() => setHeadersRevealed(true)} size="sm" type="button" variant="outline">{t("mcp.form.reveal")}</Button>
                </div>
              )}
              {fieldErrors.headers ? <span className="text-xs text-[hsl(var(--danger))]">{fieldErrors.headers}</span> : null}
            </div>
          </div>
        )}

        {error ? <div className="mt-3 rounded-md border p-3 text-sm ucd-status-danger">{error}</div> : null}

        <div className="mt-4 flex justify-end gap-2">
          <Button variant="outline" onClick={onCancel}>{t("mcp.form.cancel")}</Button>
          <Button onClick={() => void handleSubmit()} disabled={saving}>
            <Save className="h-4 w-4" aria-hidden="true" />
            {saving ? t("mcp.form.saving") : t("mcp.form.save")}
          </Button>
        </div>
    </ApplicationDialog>
  );
}
