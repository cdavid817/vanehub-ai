import { Clipboard, Download, Upload } from "lucide-react";
import { useMemo, useState } from "react";
import { useTranslation } from "react-i18next";
import { ApplicationDialog } from "../../../components/ui/application-dialog";
import { Button } from "../../../components/ui/button";
import { previewMcpImportNames } from "../../../services/mcp-import";
import type { McpImportExport, McpImportResult, McpScope, McpServerConfig } from "../../../types/mcp";
import {
  formatMcpFailure,
  mcpErrorFromUnknown,
  mcpTransportTranslationKey,
} from "./mcp-presentation";

export function McpImportExportModal({
  servers,
  onCancel,
  onImport,
  onExport,
}: {
  servers: McpServerConfig[];
  onCancel: () => void;
  onImport: (input: string, scope: McpScope) => Promise<McpImportResult>;
  onExport: (names: string[]) => Promise<McpImportExport>;
}) {
  const { t } = useTranslation();
  const [mode, setMode] = useState<"import" | "export">("import");
  const [scope, setScope] = useState<McpScope>("user");
  const [input, setInput] = useState('{\n  "mcpServers": {}\n}');
  const [selected, setSelected] = useState<string[]>(servers.map((server) => server.name));
  const [output, setOutput] = useState("");
  const [message, setMessage] = useState<string | null>(null);
  const [importResult, setImportResult] = useState<McpImportResult | null>(null);
  const [error, setError] = useState<string | null>(null);

  const importNames = useMemo(() => previewMcpImportNames(input), [input]);

  function switchMode(nextMode: "import" | "export") {
    setMode(nextMode);
    setMessage(null);
    setImportResult(null);
    setError(null);
  }

  async function handleImport() {
    setError(null);
    setMessage(null);
    setImportResult(null);
    try {
      setImportResult(await onImport(input, scope));
    } catch (err) {
      const failure = mcpErrorFromUnknown(err);
      setError(formatMcpFailure(t, failure.errorCode, failure.message));
    }
  }

  async function handleExport() {
    setError(null);
    setMessage(null);
    setImportResult(null);
    try {
      const data = await onExport(selected);
      setOutput(JSON.stringify(data, null, 2));
      setMessage(t("mcp.modal.exported", { count: Object.keys(data.mcpServers).length }));
    } catch (err) {
      const failure = mcpErrorFromUnknown(err);
      setError(formatMcpFailure(t, failure.errorCode, failure.message));
    }
  }

  async function copyOutput() {
    await navigator.clipboard.writeText(output);
    setMessage(t("mcp.modal.copied"));
  }

  return (
    <ApplicationDialog maxWidth="max-w-3xl" onClose={onCancel} title={t("mcp.importExport")}>
        <div className="mb-4 flex items-center justify-between gap-3">
          <div className="flex gap-2">
            <Button variant={mode === "import" ? "default" : "outline"} onClick={() => switchMode("import")}>
              <Upload className="h-4 w-4" aria-hidden="true" />
              {t("mcp.modal.import")}
            </Button>
            <Button variant={mode === "export" ? "default" : "outline"} onClick={() => switchMode("export")}>
              <Download className="h-4 w-4" aria-hidden="true" />
              {t("mcp.modal.export")}
            </Button>
          </div>
          <Button variant="outline" onClick={onCancel}>{t("mcp.form.close")}</Button>
        </div>

        {mode === "import" ? (
          <div className="grid gap-3">
            <label className="grid gap-1 text-sm">
              <span className="text-xs text-muted-foreground">{t("mcp.modal.importScope")}</span>
              <select className="ucd-input h-9 rounded px-3 outline-hidden focus-visible:ring-2 focus-visible:ring-ring" value={scope} onChange={(event) => setScope(event.target.value as McpScope)}>
                <option value="user">{t("mcp.scope.userConfig")}</option>
                <option value="project">{t("mcp.scope.projectConfig")}</option>
              </select>
            </label>
            <textarea
              data-dialog-autofocus
              className="ucd-input min-h-72 rounded p-3 font-mono text-xs outline-hidden focus-visible:ring-2 focus-visible:ring-ring"
              value={input}
              onChange={(event) => {
                setInput(event.target.value);
                setImportResult(null);
              }}
            />
            <div className="text-xs text-muted-foreground">{t("mcp.modal.importTransportHint")}</div>
            {importNames.length ? <div className="text-xs text-muted-foreground">{t("mcp.modal.preview", { names: importNames.join(", ") })}</div> : null}
            <Button onClick={() => void handleImport()}>
              <Upload className="h-4 w-4" aria-hidden="true" />
              {t("mcp.modal.confirmImport")}
            </Button>
          </div>
        ) : (
          <div className="grid gap-3">
            <div className="grid gap-2 md:grid-cols-2">
              {servers.map((server) => (
                <label className="flex items-center gap-2 rounded border border-border p-2 text-sm" key={server.name}>
                  <input
                    checked={selected.includes(server.name)}
                    onChange={(event) =>
                      setSelected((current) =>
                        event.target.checked ? [...current, server.name] : current.filter((name) => name !== server.name),
                      )
                    }
                    type="checkbox"
                  />
                  <span className="min-w-0 flex-1 truncate">{server.name}</span>
                  <span className="text-xs text-muted-foreground">
                    {t(mcpTransportTranslationKey(server.transportType))}
                  </span>
                </label>
              ))}
            </div>
            <div className="text-xs text-muted-foreground">{t("mcp.modal.exportTransportHint")}</div>
            <Button onClick={() => void handleExport()}>
              <Download className="h-4 w-4" aria-hidden="true" />
              {t("mcp.modal.generateJson")}
            </Button>
            <textarea readOnly className="ucd-input min-h-64 rounded p-3 font-mono text-xs outline-hidden" value={output} />
            <Button variant="outline" onClick={() => void copyOutput()} disabled={!output}>
              <Clipboard className="h-4 w-4" aria-hidden="true" />
              {t("mcp.modal.copy")}
            </Button>
          </div>
        )}

        {message ? <div className="mt-3 rounded-md border p-3 text-sm ucd-status-success">{message}</div> : null}
        {importResult ? (
          <div
            className={`mt-3 rounded-md border p-3 text-sm ${importResult.failures.length ? "ucd-status-warning" : "ucd-status-success"}`}
            role="status"
          >
            <div>
              {t("mcp.modal.importSummary", {
                imported: importResult.imported.length,
                skipped: importResult.skipped.length,
                failed: importResult.failures.length,
              })}
            </div>
            {importResult.failures.length ? (
              <ul className="mt-2 grid gap-1 text-xs">
                {importResult.failures.map((failure) => (
                  <li className="wrap-break-word" key={`${failure.stage}:${failure.name}`}>
                    <span className="font-medium">{failure.name}</span>: {t(`mcp.modal.failure.${failure.stage}`)} ·{" "}
                    {failure.errorCode
                      ? formatMcpFailure(t, failure.errorCode, failure.message)
                      : failure.message}
                  </li>
                ))}
              </ul>
            ) : null}
          </div>
        ) : null}
        {error ? <div className="mt-3 rounded-md border p-3 text-sm ucd-status-danger">{error}</div> : null}
    </ApplicationDialog>
  );
}
