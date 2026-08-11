import { useTranslation } from "react-i18next";
import { Badge } from "../../../components/ui/badge";
import type { LspLanguageId, LspServerDiscovery } from "../../../types/lsp";
import type { LspLanguageDraft } from "./lsp-configuration-form";

const inputClass = "min-h-9 w-full rounded-md border border-border bg-background px-3 py-2 text-sm focus-visible:outline-hidden focus-visible:ring-2 focus-visible:ring-ring disabled:cursor-not-allowed disabled:opacity-60";

export function LspLanguageConfigurationCard({
  discovery,
  draft,
  errorKey,
  language,
  onChange,
  pending,
}: {
  discovery?: LspServerDiscovery;
  draft: LspLanguageDraft;
  errorKey?: string;
  language: LspLanguageId;
  onChange: (draft: LspLanguageDraft) => void;
  pending: boolean;
}) {
  const { t } = useTranslation();
  const languageName = t(`lspSettings.language.${language}`);
  const optionsId = `lsp-${language}-options`;
  const optionsDescriptionId = `${optionsId}-description`;
  const optionsErrorId = `${optionsId}-error`;

  return (
    <fieldset className="rounded-lg border border-border bg-muted/15 p-4">
      <legend className="px-1 text-sm font-semibold">{languageName}</legend>
      <label className="mt-1 flex items-start gap-3 rounded-md py-2 text-sm">
        <input
          checked={draft.enabled}
          className="mt-0.5 h-4 w-4 accent-primary"
          disabled={pending}
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

      <label className="mt-4 block text-sm font-medium">
        {t("lspSettings.discovery.override")}
        <input
          aria-label={`${languageName} · ${t("lspSettings.discovery.override")}`}
          className={`${inputClass} mt-1 font-mono text-xs`}
          disabled={pending}
          onChange={(event) => onChange({ ...draft, executableOverride: event.target.value })}
          placeholder={t("lspSettings.discovery.overridePlaceholder")}
          spellCheck={false}
          value={draft.executableOverride}
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
        disabled={pending}
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
