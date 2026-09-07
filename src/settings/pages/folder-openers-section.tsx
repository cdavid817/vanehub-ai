import { RefreshCw } from "lucide-react";
import { useEffect, useState } from "react";
import { useTranslation } from "react-i18next";
import { Button } from "../../components/ui/button";
import { agentService } from "../../services/runtime-agent-client";
import type { FolderOpenerAvailability, FolderOpenerId, FolderOpenerPreferences } from "../../types/folder-opener";
import { FolderOpenerCard } from "./folder-opener-card";
import { SettingsDisclosure, SettingsRow } from "./page-parts";

export function FolderOpenersSection() {
  const { t } = useTranslation();
  const [openers, setOpeners] = useState<FolderOpenerAvailability[]>([]);
  const [preferences, setPreferences] = useState<FolderOpenerPreferences | null>(null);
  const [busy, setBusy] = useState(true);
  const [error, setError] = useState<string | null>(null);

  async function load(refresh = false) {
    setBusy(true);
    setError(null);
    try {
      const [nextOpeners, nextPreferences] = await Promise.all([
        refresh ? agentService.refreshFolderOpeners() : agentService.listFolderOpeners(),
        agentService.getFolderOpenerPreferences(),
      ]);
      setOpeners(nextOpeners);
      setPreferences(nextPreferences);
    } catch (cause) {
      setError(cause instanceof Error ? cause.message : String(cause));
    } finally {
      setBusy(false);
    }
  }

  useEffect(() => { void load(); }, []);

  async function save(configuredDefaultOpenerId: FolderOpenerId, enabledOpenerIds: FolderOpenerId[]) {
    if (!preferences) return;
    const previous = preferences;
    setPreferences({ ...preferences, configuredDefaultOpenerId, effectiveDefaultOpenerId: configuredDefaultOpenerId, enabledOpenerIds, fallbackActive: false });
    setBusy(true);
    setError(null);
    try {
      setPreferences(await agentService.saveFolderOpenerPreferences({ configuredDefaultOpenerId, enabledOpenerIds }));
    } catch (cause) {
      setPreferences(previous);
      setError(cause instanceof Error ? cause.message : String(cause));
    } finally {
      setBusy(false);
    }
  }

  function moveOpener(openerId: FolderOpenerId, direction: -1 | 1) {
    if (!preferences) return;
    const current = preferences.enabledOpenerIds;
    const index = current.indexOf(openerId);
    const target = index + direction;
    if (index < 0 || target < 0 || target >= current.length) return;
    const next = [...current];
    [next[index], next[target]] = [next[target], next[index]];
    void save(preferences.configuredDefaultOpenerId, next);
  }

  function toggleOpener(openerId: FolderOpenerId, enabled: boolean) {
    if (!preferences) return;
    const nextEnabled = enabled ? [...preferences.enabledOpenerIds, openerId] : preferences.enabledOpenerIds.filter((id) => id !== openerId);
    const nextDefault = nextEnabled.includes(preferences.configuredDefaultOpenerId) ? preferences.configuredDefaultOpenerId : "file-explorer";
    void save(nextDefault, nextEnabled);
  }

  const enabledIds = preferences?.enabledOpenerIds ?? [];
  const defaultChoices = openers.filter((item) => item.status === "available" && enabledIds.includes(item.id));
  // Enabled openers first, in their saved order, so the list reads as the same ranking the
  // session toolbar menu uses; everything else follows in catalog order.
  const orderedOpeners = enabledIds
    .map((openerId) => openers.find((item) => item.id === openerId))
    .filter((opener): opener is FolderOpenerAvailability => Boolean(opener))
    .concat(openers.filter((opener) => !enabledIds.includes(opener.id)));

  return (
    <>
      {error ? <div className="border-b border-border/70 px-5 py-3 text-xs ucd-status-danger sm:px-6">{error}</div> : null}
      <SettingsRow description={t("folderOpeners.defaultDescription")} title={t("folderOpeners.default")}>
        <label className="block text-sm">
          <span className="sr-only">{t("folderOpeners.default")}</span>
          <select
            aria-label={t("folderOpeners.default")}
            className="ucd-input h-9 w-full min-w-48 rounded-lg px-3 text-sm outline-hidden focus-visible:ring-2 focus-visible:ring-ring sm:w-auto"
            disabled={busy || !preferences || defaultChoices.length === 0}
            onChange={(event) => void save(event.target.value as FolderOpenerId, enabledIds)}
            value={defaultChoices.length === 0 ? "" : preferences?.configuredDefaultOpenerId ?? "file-explorer"}
          >
            {defaultChoices.length === 0 ? <option value="">{busy ? t("folderOpeners.detecting") : t("folderOpeners.noneAvailable")}</option> : null}
            {defaultChoices.map((item) => <option key={item.id} value={item.id}>{t(`folderOpeners.name.${item.id}`)}</option>)}
          </select>
        </label>
      </SettingsRow>
      <SettingsDisclosure description={t("folderOpeners.manageDescription")} embedded title={t("folderOpeners.manage")}>
        <div className="grid gap-4">
          <div className="grid gap-3 lg:grid-cols-2">
            {orderedOpeners.map((opener) => {
              const index = enabledIds.indexOf(opener.id);
              return (
                <FolderOpenerCard
                  busy={busy || !preferences}
                  canMoveDown={index >= 0 && index < enabledIds.length - 1}
                  canMoveUp={index > 0}
                  checked={index >= 0}
                  key={opener.id}
                  locked={opener.id === "file-explorer"}
                  onMove={(direction) => moveOpener(opener.id, direction)}
                  onToggle={(enabled) => toggleOpener(opener.id, enabled)}
                  opener={opener}
                />
              );
            })}
          </div>
          {preferences?.fallbackActive ? <div className="rounded border p-2 text-xs ucd-status-warning">{t("folderOpeners.fallbackActive")}</div> : null}
          {/* Label stays fixed while detecting so the control keeps its width; the spinner carries the state. */}
          <Button className="justify-self-start" disabled={busy} onClick={() => void load(true)} variant="outline">
            <RefreshCw className={busy ? "h-4 w-4 animate-spin" : "h-4 w-4"} aria-hidden="true" />
            {t("folderOpeners.refresh")}
          </Button>
        </div>
      </SettingsDisclosure>
    </>
  );
}
