import { useTranslation } from "react-i18next";
import { Button } from "../../../components/ui/button";
import type { LspDistribution } from "../../../types/lsp";

/**
 * Install and uninstall for a language whose server VaneHub can fetch.
 *
 * Its own component because the language card is at the 300-line rule and because this is the one
 * control that starts a network operation — keeping it separate makes "what can this page do to my
 * machine" a question with a short answer.
 */
export function LspInstallControl({
  busy,
  distribution,
  installed,
  languageName,
  onInstall,
  onUninstall,
  reasonCode,
}: {
  busy: boolean;
  distribution: LspDistribution;
  installed: boolean;
  languageName: string;
  onInstall: () => void;
  onUninstall: () => void;
  reasonCode?: string;
}) {
  const { t } = useTranslation();

  return (
    <div className="mt-4 rounded-md border border-border/70 p-3">
      <p className="text-sm font-medium">{t("lspSettings.install.title")}</p>
      <p className="mt-1 text-xs leading-5 text-muted-foreground">
        {installed ? t("lspSettings.install.installed") : t("lspSettings.install.notInstalled")}
      </p>
      {distribution.verified ? null : (
        // Said before the click, not after. Presenting an unverified download as a verified one is
        // the failure this line exists to prevent.
        <p className="mt-2 text-xs leading-5 ucd-status-warning" role="note">
          {t("lspSettings.install.unverified")}
        </p>
      )}
      {reasonCode ? (
        <p className="mt-2 text-xs ucd-status-warning">
          {t(`lspSettings.reason.${reasonCode}`, { defaultValue: reasonCode })}
        </p>
      ) : null}
      <div className="mt-3 flex gap-2">
        <Button
          aria-label={`${languageName} · ${t("lspSettings.install.action")}`}
          disabled={busy}
          onClick={onInstall}
          size="sm"
          type="button"
          variant="outline"
        >
          {busy ? t("lspSettings.install.working") : t("lspSettings.install.action")}
        </Button>
        {installed ? (
          <Button
            aria-label={`${languageName} · ${t("lspSettings.install.remove")}`}
            disabled={busy}
            onClick={onUninstall}
            size="sm"
            type="button"
            variant="outline"
          >
            {t("lspSettings.install.remove")}
          </Button>
        ) : null}
      </div>
    </div>
  );
}
