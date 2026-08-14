import { Minimize2 } from "lucide-react";
import { useState } from "react";
import { useTranslation } from "react-i18next";
import { useSettings } from "../../settings-provider";

const settingKey = "automaticContextCompactionEnabled" as const;

export function OnePieceContextCompactionSection() {
  const { t } = useTranslation();
  const { loading, saveSetting, savingKey, settings } = useSettings();
  const [error, setError] = useState<string | null>(null);
  const enabled = settings.automaticContextCompactionEnabled;
  const saving = savingKey === settingKey;

  function toggle() {
    setError(null);
    void saveSetting(settingKey, !enabled).catch((cause) => {
      setError(cause instanceof Error ? cause.message : String(cause));
    });
  }

  return (
    <section aria-labelledby="onepiece-context-compaction-heading" className="ucd-panel ucd-interactive rounded-lg p-4">
      <div className="grid gap-4 md:grid-cols-[minmax(0,1fr)_auto] md:items-center">
        <div>
          <div className="flex items-center gap-2">
            <Minimize2 aria-hidden="true" className="h-4 w-4 text-primary" />
            <h3 className="text-sm font-semibold" id="onepiece-context-compaction-heading">
              {t("onepiece.contextCompaction.title")}
            </h3>
          </div>
          <p className="mt-2 text-sm leading-6 text-muted-foreground">
            {t("onepiece.contextCompaction.description")}
          </p>
          <p className="mt-1 text-xs leading-5 text-muted-foreground">
            {t("onepiece.contextCompaction.scope")}
          </p>
        </div>
        <div className="flex items-center gap-3 md:justify-end">
          <span className="text-sm text-muted-foreground">
            {t(enabled ? "cliParameters.common.enabled" : "cliParameters.common.disabled")}
          </span>
          <button
            aria-checked={enabled}
            aria-label={t("onepiece.contextCompaction.title")}
            className={`relative h-6 w-11 shrink-0 rounded-full transition-colors focus-visible:outline-hidden focus-visible:ring-2 focus-visible:ring-ring ${enabled ? "bg-primary" : "bg-muted-foreground/40"}`}
            disabled={loading || savingKey !== null}
            onClick={toggle}
            role="switch"
            type="button"
          >
            <span className={`absolute left-1 top-1 h-4 w-4 rounded-full bg-background shadow-sm transition-transform ${enabled ? "translate-x-5" : "translate-x-0"}`} />
          </button>
        </div>
      </div>
      {saving ? <p className="mt-3 text-xs text-muted-foreground" role="status">{t("onepiece.contextCompaction.saving")}</p> : null}
      {error ? <p className="mt-3 rounded-md border p-3 text-sm ucd-status-warning" role="alert">{t("onepiece.contextCompaction.saveFailed", { message: error })}</p> : null}
    </section>
  );
}
