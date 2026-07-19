import { RefreshCw } from "lucide-react";
import { useEffect, useState } from "react";
import { useTranslation } from "react-i18next";
import { Button } from "../../components/ui/button";
import { FolderOpenerIcon } from "../../components/folder-opener-icon";
import { agentService } from "../../services/runtime-agent-client";
import type { FolderOpenerAvailability, FolderOpenerId, FolderOpenerPreferences } from "../../types/folder-opener";
import { SectionPanel } from "./page-parts";

export function FolderOpenersSection() {
  const { t } = useTranslation();
  const [openers, setOpeners] = useState<FolderOpenerAvailability[]>([]);
  const [preferences, setPreferences] = useState<FolderOpenerPreferences | null>(null);
  const [busy, setBusy] = useState(true);
  const [error, setError] = useState<string | null>(null);

  async function load(refresh = false) {
    setBusy(true); setError(null);
    try {
      const [nextOpeners, nextPreferences] = await Promise.all([
        refresh ? agentService.refreshFolderOpeners() : agentService.listFolderOpeners(),
        agentService.getFolderOpenerPreferences(),
      ]);
      setOpeners(nextOpeners); setPreferences(nextPreferences);
    } catch (cause) { setError(cause instanceof Error ? cause.message : String(cause)); }
    finally { setBusy(false); }
  }

  useEffect(() => { void load(); }, []);

  async function save(configuredDefaultOpenerId: FolderOpenerId, enabledOpenerIds: FolderOpenerId[]) {
    if (!preferences) return;
    const previous = preferences;
    setPreferences({ ...preferences, configuredDefaultOpenerId, effectiveDefaultOpenerId: configuredDefaultOpenerId, enabledOpenerIds, fallbackActive: false });
    setBusy(true); setError(null);
    try { setPreferences(await agentService.saveFolderOpenerPreferences({ configuredDefaultOpenerId, enabledOpenerIds })); }
    catch (cause) { setPreferences(previous); setError(cause instanceof Error ? cause.message : String(cause)); }
    finally { setBusy(false); }
  }

  return (
    <SectionPanel title={t("folderOpeners.title")} description={t("folderOpeners.description")}>
      <div className="grid gap-3">
        {error ? <div className="rounded border p-2 text-xs ucd-status-danger">{error}</div> : null}
        <label className="grid gap-1 text-sm">
          <span className="text-muted-foreground">{t("folderOpeners.default")}</span>
          <select className="ucd-input h-9 rounded px-3" disabled={busy || !preferences} value={preferences?.configuredDefaultOpenerId ?? "file-explorer"} onChange={(event) => void save(event.target.value as FolderOpenerId, preferences?.enabledOpenerIds ?? ["file-explorer"])}>
            {openers.filter((item) => item.status === "available" && preferences?.enabledOpenerIds.includes(item.id)).map((item) => <option key={item.id} value={item.id}>{t(`folderOpeners.name.${item.id}`)}</option>)}
          </select>
        </label>
        <div className="grid gap-2">
          {openers.map((opener) => {
            const checked = preferences?.enabledOpenerIds.includes(opener.id) ?? false;
            const locked = opener.id === "file-explorer";
            return <label className="flex items-start gap-3 rounded-lg border border-border bg-[hsl(var(--panel-muted))] p-3" key={opener.id}>
              <input checked={checked} className="mt-1" disabled={busy || locked || !preferences} onChange={(event) => {
                if (!preferences) return;
                const enabled = event.target.checked ? [...preferences.enabledOpenerIds, opener.id] : preferences.enabledOpenerIds.filter((id) => id !== opener.id);
                const nextDefault = enabled.includes(preferences.configuredDefaultOpenerId) ? preferences.configuredDefaultOpenerId : "file-explorer";
                void save(nextDefault, enabled);
              }} type="checkbox" />
              <span className="flex h-8 w-8 shrink-0 items-center justify-center rounded border border-border bg-background text-primary"><FolderOpenerIcon id={opener.id} /></span>
              <span className="min-w-0 flex-1">
                <span className="block text-sm font-medium">{t(`folderOpeners.name.${opener.id}`)}</span>
                <span className="block text-xs text-muted-foreground">{t(`folderOpeners.status.${opener.status}`)}{locked ? ` · ${t("folderOpeners.fallback")}` : ""}</span>
                {opener.version || opener.edition ? <span className="mt-1 block text-[11px] text-muted-foreground">
                  {opener.version ? `${t("folderOpeners.version")}: ${opener.version}` : ""}
                  {opener.version && opener.edition ? " · " : ""}
                  {opener.edition ? `${t("folderOpeners.edition")}: ${opener.edition}` : ""}
                </span> : null}
                {opener.executablePath ? <span className="mt-1 block truncate font-mono text-[11px] text-muted-foreground" title={opener.executablePath}>{opener.executablePath}</span> : null}
              </span>
            </label>;
          })}
        </div>
        {preferences?.fallbackActive ? <div className="rounded border p-2 text-xs ucd-status-warning">{t("folderOpeners.fallbackActive")}</div> : null}
        <Button className="justify-self-start" disabled={busy} onClick={() => void load(true)} variant="outline"><RefreshCw className={busy ? "h-4 w-4 animate-spin" : "h-4 w-4"} />{t("folderOpeners.refresh")}</Button>
      </div>
    </SectionPanel>
  );
}
