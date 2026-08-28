import { useTranslation } from "react-i18next";
import { Badge } from "../../../components/ui/badge";
import type {
  LspDistribution,
  LspLanguageId,
  LspOverrideTarget,
  LspServerDiscovery,
} from "../../../types/lsp";
import { LspInstallControl } from "./lsp-install-control";
import type { LspLanguageDraft } from "./lsp-configuration-form";

const inputClass = "min-h-9 w-full rounded-md border border-border bg-background px-3 py-2 text-sm focus-visible:outline-hidden focus-visible:ring-2 focus-visible:ring-ring disabled:cursor-not-allowed disabled:opacity-60";

export function LspLanguageConfigurationCard({
  defaultStartupArguments,
  discovery,
  draft,
  errorKey,
  install,
  language,
  onChange,
  overrideTarget,
  pending,
  prerequisite,
  supportedOnHost,
}: {
  defaultStartupArguments: string[];
  discovery?: LspServerDiscovery;
  draft: LspLanguageDraft;
  errorKey?: string;
  language: LspLanguageId;
  onChange: (draft: LspLanguageDraft) => void;
  install?: {
    busy: boolean;
    distribution: LspDistribution;
    installed: boolean;
    onInstall: () => void;
    onUninstall: () => void;
    reasonCode?: string;
  };
  overrideTarget: LspOverrideTarget;
  pending: boolean;
  prerequisite: string | null;
  supportedOnHost: boolean;
}) {
  const { t } = useTranslation();
  // Falls back to the raw id rather than rendering the missing key. Without this, registering a
  // language would blank out its label in every locale that had not been updated yet.
  const languageName = t(`lspSettings.language.${language}`, { defaultValue: language });
  const optionsId = `lsp-${language}-options`;
  const optionsDescriptionId = `${optionsId}-description`;
  const optionsErrorId = `${optionsId}-error`;
  const disabled = pending || !supportedOnHost;
  // Read from the descriptor, never from the language id. A second install-directory language has
  // to work here without an edit, which an equality check against a name would quietly give up.
  const directoryOverride = overrideTarget === "install_directory";
  const overrideLabel = directoryOverride
    ? t("lspSettings.discovery.installDirectory")
    : t("lspSettings.discovery.override");

  return (
    <fieldset className="rounded-lg border border-border bg-muted/15 p-4">
      <legend className="px-1 text-sm font-semibold">{languageName}</legend>
      {supportedOnHost ? null : (
        <p className="mt-1 rounded-md border border-border/70 p-2 text-xs ucd-status-warning" role="note">
          {t("lspSettings.language.unsupportedOnHost")}
        </p>
      )}
      {prerequisite === null ? null : (
        <p className="mt-1 rounded-md border border-border/70 p-2 text-xs text-muted-foreground" role="note">
          {t("lspSettings.language.prerequisite", { prerequisite })}
        </p>
      )}
      <label className="mt-1 flex items-start gap-3 rounded-md py-2 text-sm">
        <input
          checked={draft.enabled}
          className="mt-0.5 h-4 w-4 accent-primary"
          disabled={disabled}
          onChange={(event) => onChange({ ...draft, enabled: event.target.checked })}
          type="checkbox"
        />
        <span>{t("lspSettings.language.enabled", { language: languageName })}</span>
      </label>

      <div className="mt-3 rounded-md border border-border/70 bg-background/70 p-3 text-xs">
        <div className="flex flex-wrap items-center justify-between gap-2">
          <span className="font-medium">{t("lspSettings.discovery.automatic")}</span>
          <Badge tone={discovery?.availability === "available" ? "success" : "warning"}>
            {t(`lspSettings.discovery.${discovery?.availability ?? "unavailable"}`)}
          </Badge>
        </div>
        {discovery?.executablePath ? (
          <code className="mt-2 block break-all text-muted-foreground">{discovery.executablePath}</code>
        ) : null}
        {discovery?.reasonCode ? (
          <p className="mt-2 ucd-status-warning">{t(`lspSettings.reason.${discovery.reasonCode}`)}</p>
        ) : null}
      </div>

      {install === undefined ? null : (
        <LspInstallControl {...install} languageName={languageName} />
      )}

      <label className="mt-4 block text-sm font-medium">
        {overrideLabel}
        <input
          aria-label={`${languageName} · ${overrideLabel}`}
          className={`${inputClass} mt-1 font-mono text-xs`}
          disabled={disabled}
          onChange={(event) => onChange({ ...draft, executableOverride: event.target.value })}
          placeholder={directoryOverride
            ? t("lspSettings.discovery.installDirectoryPlaceholder")
            : t("lspSettings.discovery.overridePlaceholder")}
          spellCheck={false}
          value={draft.executableOverride}
        />
      </label>

      <label className="mt-4 block text-sm font-medium">
        {t("lspSettings.startupArguments.title")}
        <p className="mt-1 text-xs leading-5 font-normal text-muted-foreground">
          {t("lspSettings.startupArguments.description", {
            defaults: defaultStartupArguments.join(" ") || t("lspSettings.startupArguments.none"),
          })}
        </p>
        <textarea
          aria-label={`${languageName} · ${t("lspSettings.startupArguments.title")}`}
          className={`${inputClass} mt-2 min-h-16 resize-y font-mono text-xs leading-5`}
          disabled={disabled}
          onChange={(event) => onChange({ ...draft, startupArguments: event.target.value })}
          placeholder={t("lspSettings.startupArguments.placeholder")}
          spellCheck={false}
          value={draft.startupArguments}
        />
      </label>

      <label className="mt-4 block text-sm font-medium" htmlFor={optionsId}>
        {t("lspSettings.initialization.title")}
      </label>
      <p className="mt-1 text-xs leading-5 text-muted-foreground" id={optionsDescriptionId}>
        {t("lspSettings.initialization.description")}
      </p>
      <textarea
        aria-describedby={`${optionsDescriptionId}${errorKey ? ` ${optionsErrorId}` : ""}`}
        aria-invalid={Boolean(errorKey)}
        aria-label={`${languageName} · ${t("lspSettings.initialization.title")}`}
        className={`${inputClass} mt-2 min-h-32 resize-y font-mono text-xs leading-5`}
        disabled={disabled}
        id={optionsId}
        onChange={(event) => onChange({ ...draft, initializationOptions: event.target.value })}
        placeholder={t("lspSettings.initialization.placeholder")}
        spellCheck={false}
        value={draft.initializationOptions}
      />
      {errorKey ? <p className="mt-2 text-xs ucd-status-danger" id={optionsErrorId} role="alert">{t(errorKey)}</p> : null}
    </fieldset>
  );
}
