import type { TFunction } from "i18next";
import type { DiagnosticField } from "../../../ui/diagnostics/diagnostic-field";
import type {
  EngineStatus,
  FasterWhisperProfile,
  LocalMediaEngine,
  LocalMediaProfile,
  LocalMediaRuntimeStatus,
  PaddleOcrProfile,
  PythonEnvironmentDiscovery,
  SherpaOnnxTtsProfile,
} from "../../../types/local-media";

/**
 * spec.md "Copyable safe settings diagnostics" for the Local Media page (OCR/STT/TTS).
 *
 * Audited before writing this (task 12.19): this page's own doc comment already says it
 * "configures software the user installed themselves" with no download and no network access, and
 * a full read of `LocalMediaProfile`/`LocalMediaRuntimeStatus`/`PythonEnvironmentDiscovery`
 * confirms there is no credential-shaped field anywhere in this page's data model -- every value
 * below is already a local filesystem path, a host device id, a version string, a backend-pinned
 * enum/reason-code union, a plain number/boolean, or a raw timestamp, the same "safe" category
 * `cli-diagnostic-summary.ts` established. Unlike IM, there is nothing here to filter out.
 *
 * Every field traces to something `setup-overview.tsx`, `python-environment-panel.tsx`, or one of
 * the three engine cards (`engine-card.tsx`'s shared metadata block, some fields behind an
 * engine's own "Advanced settings" disclosure -- still part of the page, the same way IM's own
 * fields are only reachable once its row is expanded) already renders. Left out on purpose because
 * nothing on this page renders them anywhere: `LocalMediaProfile.revision`/`.updatedAt` and
 * `LocalMediaRuntimeStatus.profileRevision`/`EngineStatus.profileRevision` (no "revision" or "last
 * saved" text exists on this page today); `LocalMediaRuntimeStatus.pathClassifications` (a
 * shape-only, already-safe-by-design field -- but still not rendered anywhere, so including it
 * would be inventing a new surface, not reusing one); and the hook's transient `issues` map
 * (this page's own in-progress validation feedback, not a fact about the environment).
 *
 * Every per-engine label is prefixed with the engine's own name (`enginePrefixed`) so the three
 * engines' identically-named fields (Language, Compute device, Python interpreter, ...) don't
 * collide into indistinguishable lines in a flat copied summary -- the one new wrapper key this
 * file needs beyond a handful of concepts (readiness, worker state, per-engine enabled) that have
 * no existing on-page label of their own.
 */
function engineLabel(t: TFunction, engine: string, field: string): string {
  return t("localMedia.diagnostics.field.enginePrefixed", { engine, field });
}

function engineStatusFields(t: TFunction, engine: string, status: EngineStatus | undefined): DiagnosticField[] {
  const readiness = status?.readiness;
  const unavailable = readiness?.state === "unavailable" ? readiness : null;
  return [
    { label: engineLabel(t, engine, t("localMedia.diagnostics.field.readiness")), value: readiness?.state ?? null },
    { label: engineLabel(t, engine, t("localMedia.diagnostics.field.readinessCode")), value: unavailable?.code ?? null },
    { label: engineLabel(t, engine, t("localMedia.diagnostics.field.readinessField")), value: unavailable?.field ?? null },
    { label: engineLabel(t, engine, t("localMedia.diagnostics.field.workerState")), value: status?.workerState ?? null },
    { label: engineLabel(t, engine, t("localMedia.settings.meta.version")), value: status?.installedVersion ?? null },
    { label: engineLabel(t, engine, t("localMedia.settings.meta.model")), value: status?.modelIdentity ?? null },
    { label: engineLabel(t, engine, t("localMedia.settings.meta.device")), value: status?.deviceSummary ?? null },
    { label: engineLabel(t, engine, t("localMedia.settings.meta.checkedAt")), value: status?.lastCheckedAt ?? null },
  ];
}

function ocrProfileFields(t: TFunction, engine: string, profile: PaddleOcrProfile): DiagnosticField[] {
  return [
    { label: engineLabel(t, engine, t("localMedia.diagnostics.field.enabled")), value: String(profile.enabled) },
    { label: engineLabel(t, engine, t("localMedia.settings.field.pythonExecutable")), value: profile.pythonExecutable || null },
    { label: engineLabel(t, engine, t("localMedia.ocr.cpuAcceleration.label")), value: profile.cpuAcceleration },
    { label: engineLabel(t, engine, t("localMedia.settings.field.paddleXConfigPath")), value: profile.paddleXConfigPath },
    { label: engineLabel(t, engine, t("localMedia.settings.field.textDetectionModelDir")), value: profile.textDetectionModelDir },
    { label: engineLabel(t, engine, t("localMedia.settings.field.textRecognitionModelDir")), value: profile.textRecognitionModelDir },
    { label: engineLabel(t, engine, t("localMedia.settings.field.textLineOrientationModelDir")), value: profile.textLineOrientationModelDir },
    { label: engineLabel(t, engine, t("localMedia.settings.field.language")), value: profile.language || null },
    { label: engineLabel(t, engine, t("localMedia.settings.field.device")), value: profile.device },
    { label: engineLabel(t, engine, t("localMedia.settings.field.maxPdfPages")), value: String(profile.maxPdfPages) },
  ];
}

function sttProfileFields(t: TFunction, engine: string, profile: FasterWhisperProfile): DiagnosticField[] {
  return [
    { label: engineLabel(t, engine, t("localMedia.diagnostics.field.enabled")), value: String(profile.enabled) },
    { label: engineLabel(t, engine, t("localMedia.settings.field.pythonExecutable")), value: profile.pythonExecutable || null },
    { label: engineLabel(t, engine, t("localMedia.settings.field.modelDirectory")), value: profile.modelDirectory || null },
    { label: engineLabel(t, engine, t("localMedia.settings.field.device")), value: profile.device },
    { label: engineLabel(t, engine, t("localMedia.settings.field.computeType")), value: profile.computeType },
    { label: engineLabel(t, engine, t("localMedia.settings.field.language")), value: profile.language || null },
    { label: engineLabel(t, engine, t("localMedia.settings.field.vadFilter")), value: String(profile.vadFilter) },
    { label: engineLabel(t, engine, t("localMedia.settings.field.microphone")), value: profile.microphoneDeviceId },
    { label: engineLabel(t, engine, t("localMedia.settings.field.beamSize")), value: String(profile.beamSize) },
    { label: engineLabel(t, engine, t("localMedia.settings.field.maxRecordingSeconds")), value: String(profile.maxRecordingSeconds) },
  ];
}

function ttsProfileFields(t: TFunction, engine: string, profile: SherpaOnnxTtsProfile): DiagnosticField[] {
  return [
    { label: engineLabel(t, engine, t("localMedia.diagnostics.field.enabled")), value: String(profile.enabled) },
    { label: engineLabel(t, engine, t("localMedia.settings.field.pythonExecutable")), value: profile.pythonExecutable || null },
    { label: engineLabel(t, engine, t("localMedia.settings.field.modelKind")), value: profile.modelKind },
    { label: engineLabel(t, engine, t("localMedia.settings.field.modelPath")), value: profile.modelPath || null },
    { label: engineLabel(t, engine, t("localMedia.settings.field.tokensPath")), value: profile.tokensPath || null },
    { label: engineLabel(t, engine, t("localMedia.settings.field.voicesPath")), value: profile.voicesPath },
    { label: engineLabel(t, engine, t("localMedia.settings.field.vocoderPath")), value: profile.vocoderPath },
    { label: engineLabel(t, engine, t("localMedia.settings.field.lexiconPath")), value: profile.lexiconPath },
    { label: engineLabel(t, engine, t("localMedia.settings.field.dataDir")), value: profile.dataDir },
    { label: engineLabel(t, engine, t("localMedia.settings.field.dictDir")), value: profile.dictDir },
    { label: engineLabel(t, engine, t("localMedia.settings.field.ruleFsts")), value: profile.ruleFsts.length > 0 ? profile.ruleFsts.join(", ") : null },
    { label: engineLabel(t, engine, t("localMedia.settings.field.speakerId")), value: String(profile.speakerId) },
    { label: engineLabel(t, engine, t("localMedia.settings.field.speed")), value: String(profile.speed) },
    { label: engineLabel(t, engine, t("localMedia.settings.field.numThreads")), value: String(profile.numThreads) },
    { label: engineLabel(t, engine, t("localMedia.settings.field.device")), value: profile.device },
    { label: engineLabel(t, engine, t("localMedia.settings.field.outputDevice")), value: profile.outputDeviceId },
  ];
}

export function buildLocalMediaDiagnosticFields(
  profile: LocalMediaProfile,
  status: LocalMediaRuntimeStatus | null,
  discovery: PythonEnvironmentDiscovery | null,
  dirty: boolean,
  t: TFunction,
): DiagnosticField[] {
  const statusFor = (engine: LocalMediaEngine) => status?.engines.find((entry) => entry.engine === engine);
  const ocrTitle = t("localMedia.settings.ocr.shortTitle");
  const sttTitle = t("localMedia.settings.stt.shortTitle");
  const ttsTitle = t("localMedia.settings.tts.shortTitle");
  // `null` here means "discovery has not run," kept distinct from a real, reliably-known zero
  // (discovery ran and found no compatible environment) -- collapsing both to "0" would misreport
  // "checked, found none" for a page that has not actually checked yet.
  const compatibleCount = discovery
    ? discovery.candidates.filter((candidate) => candidate.compatibility === "compatible").length
    : null;
  const candidatePaths = discovery?.candidates.map((candidate) => candidate.executablePath).join(", ") ?? "";

  return [
    { label: t("localMedia.settings.overview.master"), value: String(profile.enabled) },
    { label: t("localMedia.settings.overview.changes"), value: String(dirty) },
    { label: t("localMedia.diagnostics.field.nativeAvailable"), value: status ? String(status.nativeAvailable) : null },
    { label: t("localMedia.diagnostics.field.platformSupport"), value: status?.platformSupport ?? null },
    { label: t("localMedia.diagnostics.field.pythonAvailability"), value: discovery?.availability ?? null },
    { label: t("localMedia.diagnostics.field.pythonReasonCode"), value: discovery?.reasonCode ?? null },
    { label: t("localMedia.diagnostics.field.pythonCompatibleCount"), value: compatibleCount === null ? null : String(compatibleCount) },
    { label: t("localMedia.diagnostics.field.pythonCandidatePaths"), value: candidatePaths || null },
    ...engineStatusFields(t, ocrTitle, statusFor("ocr")),
    ...ocrProfileFields(t, ocrTitle, profile.ocr),
    ...engineStatusFields(t, sttTitle, statusFor("stt")),
    ...sttProfileFields(t, sttTitle, profile.stt),
    ...engineStatusFields(t, ttsTitle, statusFor("tts")),
    ...ttsProfileFields(t, ttsTitle, profile.tts),
  ];
}
