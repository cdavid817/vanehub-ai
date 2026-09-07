import { createContext, useCallback, useContext, useEffect, useMemo, useRef, useState, type ReactNode } from "react";
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
  pickDirectory: () => Promise<string | null>;
  testNetworkProxy: (input: { url: string; bypass: string }) => Promise<NetworkProxyTestResult>;
  scanNetworkProxies: () => Promise<DetectedNetworkProxy[]>;
  reportClientLogEvent: (event: ClientLogEvent) => Promise<void>;
}

const SettingsContext = createContext<SettingsContextValue | null>(null);

type ActivateLanguage = typeof activateAppLanguage;

async function applySettings(settings: AppSettings, activateLanguage: ActivateLanguage) {
  document.documentElement.style.fontSize = settings.fontSize;
  document.documentElement.dataset.theme = settings.theme;
  // Separate attribute from `data-theme`: the CLI terminal palette is its own preference and
  // neither one may overwrite the other. Agent terminals observe this attribute and repaint.
  document.documentElement.dataset.cliTerminalTheme = settings.cliTerminalTheme;
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

  // Sequential saves (reset runs eleven in a row) must each build on the previous result, not on
  // the settings the caller was rendered with; a ref sidesteps the stale closure without forcing
  // every consumer to re-subscribe after each save.
  const settingsRef = useRef(settings);
  settingsRef.current = settings;

  const saveSetting = useCallback(
    async <K extends AppSettingKey>(key: K, value: AppSettings[K]) => {
      validateSettingValue(key, value);
      setSavingKey(key);
      setError(null);
      const previousSettings = settingsRef.current;
      const optimisticSettings = normalizeAppSettings({ ...previousSettings, [key]: value });
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
    [activateLanguage],
  );

  const setLaunchOnStartup = useCallback(
    async (enabled: boolean) => {
      setSavingKey("launchOnStartup");
      setError(null);
      const previousSettings = settingsRef.current;
      const optimisticSettings = normalizeAppSettings({ ...previousSettings, launchOnStartup: enabled });
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
    [activateLanguage],
  );

  const resetSettings = useCallback(async () => {
    const resettableKeys: AppSettingKey[] = [
      "applicationLanguage",
      "fontSize",
      "theme",
      "cliTerminalTheme",
      "defaultFolderPath",
      "logDirectory",
      "networkProxyUrl",
      "networkProxyBypass",
      "launchOnStartup",
      "defaultPolicyTemplate",
      "automaticContextCompactionEnabled",
      "contextQualityRetentionDays",
    ];
    // One key failing must not leave the rest un-reset: the user asked for all of them, so every
    // key is attempted and the failures are reported together at the end.
    const failures: string[] = [];
    for (const key of resettableKeys) {
      try {
        if (key === "launchOnStartup") {
          if (settings.launchOnStartupAvailable) await setLaunchOnStartup(defaultAppSettings.launchOnStartup);
        } else {
          await saveSetting(key, defaultAppSettings[key]);
        }
      } catch (cause) {
        failures.push(`${key}: ${cause instanceof Error ? cause.message : String(cause)}`);
      }
    }
    if (failures.length > 0) {
      const message = failures.join("\n");
      setError(message);
      throw new Error(message);
    }
  }, [saveSetting, setLaunchOnStartup, settings.launchOnStartupAvailable]);

  const getDataManagementInfo = useCallback(async () => {
    return settingsService.getDataManagementInfo();
  }, []);

  const openDatabaseDirectory = useCallback(async () => {
    await settingsService.openDatabaseDirectory();
  }, []);

  const openLogDirectory = useCallback(async () => {
    await settingsService.openLogDirectory();
  }, []);

  const pickDirectory = useCallback(async () => {
    return settingsService.pickDirectory();
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
      pickDirectory,
      testNetworkProxy,
      scanNetworkProxies,
      reportClientLogEvent,
    }),
    [error, getDataManagementInfo, loading, nodeInfo, openDatabaseDirectory, openLogDirectory, pickDirectory, refreshNodeInfo, reportClientLogEvent, resetSettings, saveSetting, scanNetworkProxies, setLaunchOnStartup, savingKey, settings, testNetworkProxy],
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
