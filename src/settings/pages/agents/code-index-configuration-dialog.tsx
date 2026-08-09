import { useState } from "react";
import { useTranslation } from "react-i18next";
import { ApplicationDialog } from "../../../components/ui/application-dialog";
import { Button } from "../../../components/ui/button";
import { normalizeCodeIndexConfiguration } from "../../../services/code-index-contract";
import { codeIndexLanguages, type CodeIndexConfigurationInput, type CodeIndexLanguage, type CodeIndexWorkspace } from "../../../types/code-index";

const inputClass = "ucd-input min-h-9 w-full rounded-md px-3 py-2 text-sm outline-hidden focus-visible:ring-2 focus-visible:ring-ring";

function initialConfiguration(workspace: CodeIndexWorkspace): CodeIndexConfigurationInput {
  return {
    enabled: workspace.enabled,
    mode: workspace.mode,
    selectedRoots: [...workspace.selectedRoots],
    languages: [...workspace.languages],
    exclusionPatterns: [...workspace.exclusionPatterns],
    maxFileBytes: workspace.maxFileBytes,
  };
}

export function CodeIndexConfigurationDialog({ workspace, pending, embeddingSource, embeddingModel, onClose, onSave }: {
  workspace: CodeIndexWorkspace;
  pending: boolean;
  embeddingSource: string | null;
  embeddingModel: string | null;
  onClose: () => void;
  onSave: (configuration: CodeIndexConfigurationInput) => Promise<void>;
}) {
  const { t } = useTranslation();
  const [configuration, setConfiguration] = useState(() => initialConfiguration(workspace));
  const [roots, setRoots] = useState(workspace.selectedRoots.join("\n"));
  const [patterns, setPatterns] = useState(workspace.exclusionPatterns.join("\n"));
  const [error, setError] = useState<string | null>(null);

  function toggleLanguage(language: CodeIndexLanguage) {
    setConfiguration((current) => ({
      ...current,
      languages: current.languages.includes(language)
        ? current.languages.filter((candidate) => candidate !== language)
        : [...current.languages, language],
    }));
  }

  async function submit() {
    try {
      const normalized = normalizeCodeIndexConfiguration({
        ...configuration,
        selectedRoots: roots.split(/\r?\n/),
        exclusionPatterns: patterns.split(/\r?\n/).filter(Boolean),
      });
      setError(null);
      await onSave(normalized);
    } catch (reason) {
      setError(reason instanceof Error ? reason.message : String(reason));
    }
  }

  return (
    <ApplicationDialog closeDisabled={pending} description={workspace.canonicalRoot} onClose={onClose} title={t("codeIndex.configuration.title", { name: workspace.displayName })}>
      <div className="space-y-5">
        <label className="flex items-start gap-3 rounded-md border border-border p-3 text-sm">
          <input checked={configuration.enabled} className="mt-0.5 h-4 w-4 accent-primary" onChange={(event) => setConfiguration((current) => ({ ...current, enabled: event.target.checked }))} type="checkbox" />
          <span><span className="block font-medium">{t("codeIndex.configuration.enabled")}</span><span className="mt-0.5 block text-xs leading-5 text-muted-foreground">{t("codeIndex.configuration.enabledHint")}</span></span>
        </label>

        <fieldset>
          <legend className="text-sm font-medium">{t("codeIndex.configuration.mode")}</legend>
          <div className="mt-2 grid gap-2 sm:grid-cols-2">
            {(["local", "semantic"] as const).map((mode) => (
              <label className="flex items-start gap-3 rounded-md border border-border px-3 py-3 text-sm" key={mode}>
                <input checked={configuration.mode === mode} className="mt-0.5 h-4 w-4 accent-primary" name="code-index-mode" onChange={() => setConfiguration((current) => ({ ...current, mode }))} type="radio" />
                <span>
                  <span className="block font-medium">{t(`codeIndex.mode.${mode}`)}</span>
                  <span className="mt-0.5 block text-xs leading-5 text-muted-foreground">{t(`codeIndex.mode.${mode}Description`)}</span>
                </span>
              </label>
            ))}
          </div>
          {configuration.mode === "semantic" ? (
            <p className="mt-2 text-xs leading-5 text-muted-foreground">
              {embeddingSource && embeddingModel
                ? t("codeIndex.configuration.semanticConfigured", { source: embeddingSource, model: embeddingModel })
                : t("codeIndex.configuration.semanticMissing")}
            </p>
          ) : null}
        </fieldset>

        <fieldset>
          <legend className="text-sm font-medium">{t("codeIndex.configuration.languages")}</legend>
          <div className="mt-2 grid grid-cols-2 gap-2 sm:grid-cols-4">
            {codeIndexLanguages.map((language) => (
              <label className="flex items-center gap-2 rounded-md border border-border px-3 py-2 text-sm" key={language}>
                <input checked={configuration.languages.includes(language)} className="h-4 w-4 accent-primary" onChange={() => toggleLanguage(language)} type="checkbox" />
                {t(`codeIndex.language.${language}`)}
              </label>
            ))}
          </div>
        </fieldset>

        <label className="block text-sm font-medium">{t("codeIndex.configuration.roots")}
          <textarea className={`${inputClass} mt-1 min-h-20 font-mono text-xs`} onChange={(event) => setRoots(event.target.value)} placeholder={t("codeIndex.configuration.rootsPlaceholder")} value={roots} />
          <span className="mt-1 block text-xs text-muted-foreground">{t("codeIndex.configuration.rootsHint")}</span>
        </label>

        <label className="block text-sm font-medium">{t("codeIndex.configuration.exclusions")}
          <textarea className={`${inputClass} mt-1 min-h-24 font-mono text-xs`} onChange={(event) => setPatterns(event.target.value)} placeholder={t("codeIndex.configuration.exclusionsPlaceholder")} value={patterns} />
          <span className="mt-1 block text-xs text-muted-foreground">{t("codeIndex.configuration.exclusionsHint")}</span>
        </label>

        <label className="block text-sm font-medium">{t("codeIndex.configuration.maxSize")}
          <input className={`${inputClass} mt-1`} max={10 * 1024} min={1} onChange={(event) => setConfiguration((current) => ({ ...current, maxFileBytes: Number(event.target.value) * 1024 }))} type="number" value={configuration.maxFileBytes / 1024} />
        </label>

        {error ? <p className="text-sm ucd-status-warning" role="alert">{error}</p> : null}
        <div className="flex flex-col-reverse gap-2 sm:flex-row sm:justify-end">
          <Button disabled={pending} onClick={onClose} variant="outline">{t("agents.edit.cancel")}</Button>
          <Button data-dialog-autofocus disabled={pending} onClick={() => void submit()}>{pending ? t("agents.edit.saving") : t("codeIndex.configuration.save")}</Button>
        </div>
      </div>
    </ApplicationDialog>
  );
}
