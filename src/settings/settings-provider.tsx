import { createContext, useCallback, useContext, useEffect, useMemo, useState, type ReactNode } from "react";
import { i18n } from "../i18n";
import { settingsService } from "../services/runtime-settings-client";
import { defaultAppSettings, normalizeAppSettings, validateSettingValue } from "../services/settings-service";
import type { AppSettingKey, AppSettings, ClientLogEvent, NodeInfo } from "../types/settings";

interface SettingsContextValue {
  settings: AppSettings;
  nodeInfo: NodeInfo | null;
  loading: boolean;
  savingKey: AppSettingKey | null;
  error: string | null;
  saveSetting: <K extends AppSettingKey>(key: K, value: AppSettings[K]) => Promise<void>;
  resetSettings: () => Promise<void>;
  refreshNodeInfo: () => Promise<void>;
  openLogDirectory: () => Promise<void>;
  reportClientLogEvent: (event: ClientLogEvent) => Promise<void>;
}

const SettingsContext = createContext<SettingsContextValue | null>(null);

function applySettings(settings: AppSettings) {
  void i18n.changeLanguage(settings.applicationLanguage);
  document.documentElement.style.fontSize = settings.fontSize;
  document.documentElement.dataset.theme = settings.theme;
}

export function SettingsProvider({ children }: { children: ReactNode }) {
  const [settings, setSettings] = useState<AppSettings>(defaultAppSettings);
  const [nodeInfo, setNodeInfo] = useState<NodeInfo | null>(null);
  const [loading, setLoading] = useState(true);
  const [savingKey, setSavingKey] = useState<AppSettingKey | null>(null);
  const [error, setError] = useState<string | null>(null);

  const refreshNodeInfo = useCallback(async () => {
    try {
      setNodeInfo(await settingsService.getNodeInfo());
    } catch (err) {
      const message = err instanceof Error ? err.message : String(err);
      setNodeInfo({ available: false, path: null, version: null, reason: message });
    }
  }, []);

  useEffect(() => {
    let cancelled = false;
    async function loadSettings() {
      try {
        const loadedSettings = normalizeAppSettings(await settingsService.getSettings());
        if (cancelled) return;
        setSettings(loadedSettings);
        applySettings(loadedSettings);
        setError(null);
      } catch (err) {
        if (cancelled) return;
        const fallback = defaultAppSettings;
        setSettings(fallback);
        applySettings(fallback);
        setError(err instanceof Error ? err.message : String(err));
      } finally {
        if (!cancelled) setLoading(false);
      }
    }
    void loadSettings();
    void refreshNodeInfo();
    return () => {
      cancelled = true;
    };
  }, [refreshNodeInfo]);

  const saveSetting = useCallback(
    async <K extends AppSettingKey>(key: K, value: AppSettings[K]) => {
      validateSettingValue(key, value);
      setSavingKey(key);
      setError(null);
      const previousSettings = settings;
      const optimisticSettings = normalizeAppSettings({ ...settings, [key]: value });
      setSettings(optimisticSettings);
      applySettings(optimisticSettings);
      try {
        const nextSettings = normalizeAppSettings(await settingsService.saveSetting({ key, value }));
        setSettings(nextSettings);
        applySettings(nextSettings);
      } catch (err) {
        setSettings(previousSettings);
        applySettings(previousSettings);
        setError(err instanceof Error ? err.message : String(err));
        throw err;
      } finally {
        setSavingKey(null);
      }
    },
    [settings],
  );

  const resetSettings = useCallback(async () => {
    const resettableKeys: AppSettingKey[] = ["applicationLanguage", "fontSize", "theme", "defaultFolderPath", "logDirectory"];
    for (const key of resettableKeys) {
      await saveSetting(key, defaultAppSettings[key]);
    }
  }, [saveSetting]);

  const openLogDirectory = useCallback(async () => {
    await settingsService.openLogDirectory();
  }, []);

  const reportClientLogEvent = useCallback(async (event: ClientLogEvent) => {
    await settingsService.reportClientLogEvent(event);
  }, []);

  const value = useMemo(
    () => ({
      settings,
      nodeInfo,
      loading,
      savingKey,
      error,
      saveSetting,
      resetSettings,
      refreshNodeInfo,
      openLogDirectory,
      reportClientLogEvent,
    }),
    [error, loading, nodeInfo, openLogDirectory, refreshNodeInfo, reportClientLogEvent, resetSettings, saveSetting, savingKey, settings],
  );

  return <SettingsContext.Provider value={value}>{children}</SettingsContext.Provider>;
}

export function useSettings() {
  const value = useContext(SettingsContext);
  if (!value) {
    throw new Error("useSettings must be used inside SettingsProvider");
  }
  return value;
}
