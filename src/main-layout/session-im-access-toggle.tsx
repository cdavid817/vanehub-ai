import { useState } from "react";
import { useTranslation } from "react-i18next";
import { Button } from "../components/ui/button";
import type { ImSessionBinding } from "../contracts/im";

export function SessionImAccessToggle({
  binding,
  enabled,
  onChange,
  pending,
  platformName,
}: {
  binding: ImSessionBinding | null;
  enabled: boolean;
  onChange: (enabled: boolean) => Promise<unknown>;
  pending: boolean;
  platformName: string;
}) {
  const { t } = useTranslation();
  const [confirmDisable, setConfirmDisable] = useState(false);

  const requestChange = (next: boolean) => {
    if (!next && binding) {
      setConfirmDisable(true);
      return;
    }
    void onChange(next);
  };

  return (
    <section className="ucd-muted-panel grid gap-3 rounded-lg p-3" data-testid="session-im-access">
      <label className="flex items-start justify-between gap-3">
        <span>
          <span className="block text-sm font-semibold">{t("im.session.access.title", { platform: platformName })}</span>
          <span className="mt-1 block text-xs text-muted-foreground">
            {t(enabled ? "im.session.access.enabledHint" : "im.session.access.disabledHint", { platform: platformName })}
          </span>
        </span>
        <input
          aria-label={t("im.session.access.switch", { platform: platformName })}
          checked={enabled}
          disabled={pending}
          onChange={(event) => requestChange(event.target.checked)}
          role="switch"
          type="checkbox"
        />
      </label>
      {confirmDisable ? (
        <div aria-live="polite" className="grid gap-2 rounded-md border border-destructive/40 p-2 text-xs">
          <p>{t("im.session.access.disableConfirm", { platform: platformName })}</p>
          <div className="grid grid-cols-2 gap-2">
            <Button onClick={() => setConfirmDisable(false)} size="sm" variant="ghost">
              {t("im.session.access.keepEnabled")}
            </Button>
            <Button
              className="bg-destructive text-destructive-foreground hover:bg-destructive/90"
              disabled={pending}
              onClick={() => void onChange(false).then(() => setConfirmDisable(false))}
              size="sm"
            >
              {t("im.session.access.confirmDisable")}
            </Button>
          </div>
        </div>
      ) : null}
    </section>
  );
}
