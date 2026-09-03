import { createContext, useCallback, useContext, useEffect, useMemo, useState, type ReactNode } from "react";
import { activateAppLanguage, i18n } from "../i18n";
import { settingsService } from "../services/runtime-settings-client";
import { defaultAppSettings, normalizeAppSettings, validateSettingValue } from "../services/settings-service";
import type { AppSettingKey, AppSettings, ClientLogEvent, DataManagementInfo, DetectedNetworkProxy, NetworkProxyTestResult, NodeInfo } from "../types/settings";

interface SettingsContextValue {
  settings: AppSettings;
  nodeInfo: NodeInfo | null;
  loading: boolean;
  savingKey: AppSettingKey | null;
  error: string | null;
  saveSetting: <K extends AppSettingKey>(key: K, value: AppSettings[K]) => Promise<void>;
  setLaunchOnStartup: (enabled: boolean) => Promise<void>;
  resetSettings: () => Promise<void>;
  refreshNodeInfo: () => Promise<void>;
  getDataManagementInfo: () => Promise<DataManagementInfo>;
  openDatabaseDirectory: () => Promise<void>;
  openLogDirectory: () => Promise<void>;
  testNetworkProxy: (input: { url: string; bypass: string }) => Promise<NetworkProxyTestResult>;
  scanNetworkProxies: () => Promise<DetectedNetworkProxy[]>;
  reportClientLogEvent: (event: ClientLogEvent) => Promise<void>;
}

const SettingsContext = createContext<SettingsContextValue | null>(null);

type ActivateLanguage = typeof activateAppLanguage;

async function applySettings(settings: AppSettings, activateLanguage: ActivateLanguage) {
  document.documentElement.style.fontSize = settings.fontSize;
  document.documentElement.dataset.theme = settings.theme;
  await activateLanguage(settings.applicationLanguage);
}

export function SettingsProvider({ children, activateLanguage = activateAppLanguage }: { children: ReactNode; activateLanguage?: ActivateLanguage }) {
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
        let loadedSettings: AppSettings;
        try {
          loadedSettings = normalizeAppSettings(await settingsService.getSettings());
        } catch (err) {
          if (cancelled) return;
          const message = err instanceof Error ? err.message : String(err);
          const fallback = defaultAppSettings;
          await applySettings(fallback, activateLanguage);
          if (cancelled) return;
          setSettings(fallback);
          setError(i18n.t("basic.settingsLoadError"));
          void settingsService.reportClientLogEvent({
            level: "error",
            kind: "critical-operation-failure",
            message,
            source: "SettingsProvider.loadSettings",
          }).catch(() => undefined);
          return;
        }
        if (cancelled) return;
        await applySettings(loadedSettings, activateLanguage);
        if (cancelled) return;
        setSettings(loadedSettings);
        setError(null);
      } catch (err) {
        if (cancelled) return;
        const fallback = defaultAppSettings;
        await applySettings(fallback, activateLanguage);
        if (cancelled) return;
        setSettings(fallback);
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
  }, [activateLanguage, refreshNodeInfo]);

  useEffect(() => {
    let active = true;
    let unsubscribe: (() => void) | undefined;
    void settingsService
      .subscribeSettingsEvents(async () => {
        try {
          const nextSettings = normalizeAppSettings(await settingsService.getSettings());
          if (!active) return;
          await applySettings(nextSettings, activateLanguage);
          if (!active) return;
          setSettings(nextSettings);
          setError(null);
        } catch (err) {
          if (active) setError(err instanceof Error ? err.message : String(err));
        }
      })
      .then((cleanup) => {
        if (active) unsubscribe = cleanup;
        else cleanup();
      });
    return () => {
      active = false;
      unsubscribe?.();
    };
  }, [activateLanguage]);

  const saveSetting = useCallback(
    async <K extends AppSettingKey>(key: K, value: AppSettings[K]) => {
      validateSettingValue(key, value);
      setSavingKey(key);
      setError(null);
      const previousSettings = settings;
      const optimisticSettings = normalizeAppSettings({ ...settings, [key]: value });
      try {
        await applySettings(optimisticSettings, activateLanguage);
        setSettings(optimisticSettings);
        const nextSettings = normalizeAppSettings(
          // The revision this screen was rendered from, not one re-read at save time. Sending a
          // fresh read would make every write succeed and the check meaningless.
          await settingsService.saveSetting({ key, value, expectedPersonalizationRevision: previousSettings.personalizationRevision }),
        );
        await applySettings(nextSettings, activateLanguage);
        setSettings(nextSettings);
      } catch (err) {
        setSettings(previousSettings);
        await applySettings(previousSettings, activateLanguage);
        setError(err instanceof Error ? err.message : String(err));
        throw err;
      } finally {
        setSavingKey(null);
      }
    },
    [activateLanguage, settings],
  );

  const resetSettings = useCallback(async () => {
    const resettableKeys: AppSettingKey[] = [
      "applicationLanguage",
      "fontSize",
      "theme",
      "defaultFolderPath",
      "logDirectory",
      "networkProxyUrl",
      "networkProxyBypass",
      "launchOnStartup",
      "defaultPolicyTemplate",
      "automaticContextCompactionEnabled",
      "contextQualityRetentionDays",
    ];
    for (const key of resettableKeys) {
      if (key === "launchOnStartup") await settingsService.setLaunchOnStartup(defaultAppSettings.launchOnStartup);
      else await saveSetting(key, defaultAppSettings[key]);
    }
  }, [saveSetting]);

  const setLaunchOnStartup = useCallback(
    async (enabled: boolean) => {
      setSavingKey("launchOnStartup");
      setError(null);
      const previousSettings = settings;
      const optimisticSettings = normalizeAppSettings({ ...settings, launchOnStartup: enabled });
      try {
        await applySettings(optimisticSettings, activateLanguage);
        setSettings(optimisticSettings);
        const nextSettings = normalizeAppSettings(await settingsService.setLaunchOnStartup(enabled));
        await applySettings(nextSettings, activateLanguage);
        setSettings(nextSettings);
      } catch (err) {
        setSettings(previousSettings);
        await applySettings(previousSettings, activateLanguage);
        setError(err instanceof Error ? err.message : String(err));
        throw err;
      } finally {
        setSavingKey(null);
      }
    },
    [activateLanguage, settings],
  );

  const getDataManagementInfo = useCallback(async () => {
    return settingsService.getDataManagementInfo();
  }, []);

  const openDatabaseDirectory = useCallback(async () => {
    await settingsService.openDatabaseDirectory();
  }, []);

  const openLogDirectory = useCallback(async () => {
    await settingsService.openLogDirectory();
  }, []);

  const testNetworkProxy = useCallback(async (input: { url: string; bypass: string }) => {
    return settingsService.testNetworkProxy(input);
  }, []);

  const scanNetworkProxies = useCallback(async () => {
    return settingsService.scanNetworkProxies();
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
      setLaunchOnStartup,
      resetSettings,
      refreshNodeInfo,
      getDataManagementInfo,
      openDatabaseDirectory,
      openLogDirectory,
      testNetworkProxy,
      scanNetworkProxies,
      reportClientLogEvent,
    }),
    [error, getDataManagementInfo, loading, nodeInfo, openDatabaseDirectory, openLogDirectory, refreshNodeInfo, reportClientLogEvent, resetSettings, saveSetting, scanNetworkProxies, setLaunchOnStartup, savingKey, settings, testNetworkProxy],
  );

  if (loading) return null;
  return <SettingsContext.Provider value={value}>{children}</SettingsContext.Provider>;
}

export function useSettings() {
  const value = useContext(SettingsContext);
  if (!value) {
    throw new Error("useSettings must be used inside SettingsProvider");
  }
  return value;
}
