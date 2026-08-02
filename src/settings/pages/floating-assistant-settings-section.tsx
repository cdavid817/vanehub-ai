import { Bot, Sparkles } from "lucide-react";
import { useEffect, useState } from "react";
import { useTranslation } from "react-i18next";
import { floatingAssistantService } from "../../services/runtime-floating-assistant-client";
import type { FloatingAssistantConfig, FloatingAssistantRuntimeInfo } from "../../types/floating-assistant";
import { useSettings } from "../settings-provider";
import { SettingsRow } from "./page-parts";

export function FloatingAssistantSettingsSection() {
  const { t } = useTranslation();
  const { reportClientLogEvent } = useSettings();
  const [runtime, setRuntime] = useState<FloatingAssistantRuntimeInfo | null>(null);
  const [config, setConfig] = useState<FloatingAssistantConfig | null>(null);
  const [saving, setSaving] = useState(false);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    let active = true;
    let cleanup: (() => void) | undefined;
    void Promise.all([floatingAssistantService.getRuntimeInfo(), floatingAssistantService.getConfig()])
      .then(([nextRuntime, nextConfig]) => {
        if (!active) return;
        setRuntime(nextRuntime);
        setConfig(nextConfig);
      })
      .catch((cause) => {
        if (active) setError(cause instanceof Error ? cause.message : String(cause));
      });
    void floatingAssistantService.subscribeEvents((event) => {
      if (active && event.kind === "configuration-changed") setConfig(event.config);
    }).then((unsubscribe) => {
      if (active) cleanup = unsubscribe;
      else unsubscribe();
    });
    return () => {
      active = false;
      cleanup?.();
    };
  }, []);

  function setEnabled(enabled: boolean) {
    setSaving(true);
    setError(null);
    void floatingAssistantService.setEnabled(enabled)
      .then(setConfig)
      .catch((cause) => {
        const message = cause instanceof Error ? cause.message : String(cause);
        setError(message);
        void reportClientLogEvent({
          level: "error",
          kind: "critical-operation-failure",
          message,
          source: "FloatingAssistantSettingsSection.setEnabled",
        });
      })
      .finally(() => setSaving(false));
  }

  return (
    <>
      <SettingsRow description={t("floating.enableHint")} title={t("floating.enable")}>
        <div className="flex items-center justify-end gap-3">
          <span className="relative flex h-8 w-8 items-center justify-center rounded-lg bg-[hsl(var(--nav-active-soft))] text-primary">
            <Bot className="h-4 w-4" aria-hidden="true" />
            <Sparkles className="absolute right-0.5 top-0.5 h-2.5 w-2.5 text-primary" aria-hidden="true" />
          </span>
          <button
            aria-checked={config?.enabled ?? false}
            aria-label={t("floating.enable")}
            className={`relative h-6 w-11 rounded-full transition-colors ${(config?.enabled ?? false) ? "bg-primary" : "bg-muted-foreground/40"}`}
            disabled={!runtime?.nativeAvailable || saving || !config}
            onClick={() => setEnabled(!(config?.enabled ?? false))}
            role="switch"
            type="button"
          >
            <span className={`absolute left-1 top-1 h-4 w-4 rounded-full bg-background shadow-sm transition-transform ${(config?.enabled ?? false) ? "translate-x-5" : "translate-x-0"}`} />
          </button>
        </div>
      </SettingsRow>
      <div className="border-b border-border/70 px-5 py-3 last:border-b-0 sm:px-6">
        <p className="text-xs leading-5 text-muted-foreground">
          {runtime?.nativeAvailable ? t("floating.windowsAvailable") : t("floating.windowsOnly")}
        </p>
        {error ? <p className="rounded-md bg-[hsl(var(--danger-soft))] p-2 text-xs text-[hsl(var(--danger))]">{error}</p> : null}
      </div>
    </>
  );
}
