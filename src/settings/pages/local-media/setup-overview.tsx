import { useTranslation } from "react-i18next";

import type { LocalMediaEngine, LocalMediaProfile, LocalMediaRuntimeStatus, PythonEnvironmentDiscovery } from "../../../types/local-media";
import { CopyDiagnosticsButton } from "../../../ui/diagnostics/CopyDiagnosticsButton";
import { SectionPanel, StatusPill } from "../page-parts";
import { buildLocalMediaDiagnosticFields } from "./local-media-diagnostic-summary";

function configured(profile: LocalMediaProfile, engine: LocalMediaEngine) {
  if (!profile[engine].pythonExecutable) return false;
  if (engine === "ocr") {
    return Boolean(profile.ocr.paddleXConfigPath || (profile.ocr.textDetectionModelDir && profile.ocr.textRecognitionModelDir));
  }
  if (engine === "stt") return Boolean(profile.stt.modelDirectory);
  return Boolean(profile.tts.modelPath && profile.tts.tokensPath);
}

function nextStep(profile: LocalMediaProfile, status: LocalMediaRuntimeStatus | null, compatible: number, dirty: boolean) {
  const enabled = (["ocr", "stt", "tts"] as const).filter((engine) => profile[engine].enabled);
  if (compatible === 0 && enabled.some((engine) => !profile[engine].pythonExecutable)) return "python";
  if (enabled.some((engine) => !configured(profile, engine))) return "configure";
  if (dirty) return "save";
  const ready = new Set(status?.engines.filter((engine) => engine.readiness.state === "ready").map((engine) => engine.engine));
  if (enabled.some((engine) => !ready.has(engine))) return "probe";
  return "complete";
}

export function SetupOverview({
  dirty,
  discovery,
  profile,
  status,
}: {
  dirty: boolean;
  discovery: PythonEnvironmentDiscovery | null;
  profile: LocalMediaProfile;
  status: LocalMediaRuntimeStatus | null;
}) {
  const { t } = useTranslation();
  const compatible = discovery?.candidates.filter((item) => item.compatibility === "compatible").length ?? 0;
  const configuredCount = (["ocr", "stt", "tts"] as const).filter((engine) => configured(profile, engine)).length;
  const readyCount = status?.engines.filter((engine) => engine.readiness.state === "ready").length ?? 0;
  const guidance = nextStep(profile, status, compatible, dirty);
  const diagnosticFields = buildLocalMediaDiagnosticFields(profile, status, discovery, dirty, t);
  const entries = [
    { label: "master", value: profile.enabled ? "on" : "off", tone: profile.enabled ? "success" : "muted" },
    { label: "python", value: compatible > 0 ? "available" : "needed", tone: compatible > 0 ? "success" : "warning" },
    { label: "configured", value: `${configuredCount}/3`, tone: configuredCount === 3 ? "success" : "warning" },
    { label: "ready", value: `${readyCount}/3`, tone: readyCount > 0 ? "success" : "muted" },
    { label: "changes", value: dirty ? "unsaved" : "saved", tone: dirty ? "warning" : "success" },
  ] as const;

  return (
    <SectionPanel description={t("localMedia.settings.overview.description")} title={t("localMedia.settings.overview.title")} variant="settings">
      <div className="grid gap-3 px-5 py-4 sm:grid-cols-2 sm:px-6 lg:grid-cols-5">
        {entries.map((entry) => (
          <div className="flex min-w-0 items-center justify-between gap-2 rounded-lg border border-border px-3 py-2" key={entry.label}>
            <span className="text-xs text-muted-foreground">{t(`localMedia.settings.overview.${entry.label}`)}</span>
            <StatusPill status={entry.value.includes("/") ? entry.value : t(`localMedia.settings.overview.${entry.value}`)} tone={entry.tone} />
          </div>
        ))}
      </div>
      <p className="border-t border-border px-5 py-3 text-sm text-muted-foreground sm:px-6" data-testid="local-media-next-step">
        <span className="font-medium text-foreground">{t("localMedia.settings.overview.nextStep")}: </span>
        {t(`localMedia.settings.overview.next.${guidance}`)}
      </p>
      <div className="flex justify-end border-t border-border px-5 py-3 sm:px-6">
        <CopyDiagnosticsButton fields={diagnosticFields} />
      </div>
    </SectionPanel>
  );
}
