import { Power } from "lucide-react";
import { useTranslation } from "react-i18next";
import { useSettings } from "../settings-provider";
import { SectionPanel } from "./page-parts";

export function StartupSettingsSection() {
  const { t } = useTranslation();
  const { loading, reportClientLogEvent, savingKey, setLaunchOnStartup, settings } = useSettings();
  const enabled = settings.launchOnStartup;
  const nativeAvailable = settings.loggingPolicy.canOpenDirectory;
  const busy = loading || savingKey === "launchOnStartup";

  function toggle() {
    void setLaunchOnStartup(!enabled).catch((cause) => {
      const message = cause instanceof Error ? cause.message : String(cause);
      void reportClientLogEvent({
        level: "error",
        kind: "critical-operation-failure",
        message,
        source: "StartupSettingsSection.setLaunchOnStartup",
      });
    });
  }

  return (
    <SectionPanel icon={Power} title={t("basic.systemBehavior")} description={t("basic.systemBehaviorDesc")}>
      <div className="flex flex-col gap-3 rounded-md border border-border bg-[hsl(var(--panel-muted))] p-4 sm:flex-row sm:items-center">
        <span className="flex h-10 w-10 shrink-0 items-center justify-center rounded-md border border-border bg-background text-primary">
          <Power className="h-5 w-5" aria-hidden="true" />
        </span>
        <div className="min-w-0 flex-1">
          <div className="text-sm font-medium text-foreground">{t("basic.launchOnStartup")}</div>
          <div className="mt-1 text-sm leading-6 text-muted-foreground">
            {nativeAvailable ? t("basic.launchOnStartupHint") : t("basic.launchOnStartupUnavailable")}
          </div>
        </div>
        <button
          aria-checked={enabled}
          aria-label={t("basic.launchOnStartup")}
          className={`relative h-6 w-11 shrink-0 rounded-full transition-colors ${enabled ? "bg-primary" : "bg-muted-foreground/40"}`}
          disabled={busy || !nativeAvailable}
          onClick={toggle}
          role="switch"
          type="button"
        >
          <span className={`absolute left-1 top-1 h-4 w-4 rounded-full bg-background shadow transition-transform ${enabled ? "translate-x-5" : "translate-x-0"}`} />
        </button>
      </div>
    </SectionPanel>
  );
}
