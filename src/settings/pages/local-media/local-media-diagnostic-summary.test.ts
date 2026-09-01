import type { TFunction } from "i18next";
import { beforeAll, describe, expect, it } from "vitest";
import { activateAppLanguage, i18n } from "../../../i18n";
import type {
  LocalMediaProfile,
  LocalMediaRuntimeStatus,
  PythonEnvironmentDiscovery,
} from "../../../types/local-media";
import { buildLocalMediaDiagnosticFields } from "./local-media-diagnostic-summary";

let t: TFunction;
beforeAll(async () => {
  await activateAppLanguage("en");
  t = i18n.getFixedT("en");
});

/** Mirrors the source file's own `engineLabel` composition, so assertions track real keys rather
 *  than hardcoded translated text. */
function engineLabel(engineKey: string, fieldKey: string): string {
  return t("localMedia.diagnostics.field.enginePrefixed", { engine: t(engineKey), field: t(fieldKey) });
}

function profile(overrides: Partial<LocalMediaProfile> = {}): LocalMediaProfile {
  return {
    profileId: "default",
    revision: 4,
    enabled: true,
    ocr: {
      enabled: true,
      pythonExecutable: "/opt/ocr/bin/python",
      cpuAcceleration: "library-default",
      paddleXConfigPath: null,
      textDetectionModelDir: "/models/det",
      textRecognitionModelDir: "/models/rec",
      textLineOrientationModelDir: null,
      language: "ch",
      device: "auto",
      maxPdfPages: 20,
    },
    stt: {
      enabled: false,
      pythonExecutable: "",
      modelDirectory: "",
      device: "auto",
      computeType: "auto",
      language: "auto",
      vadFilter: true,
      beamSize: 5,
      microphoneDeviceId: null,
      maxRecordingSeconds: 120,
    },
    tts: {
      enabled: false,
      pythonExecutable: "",
      modelKind: "vits",
      modelPath: "",
      tokensPath: "",
      lexiconPath: null,
      dataDir: null,
      dictDir: null,
      voicesPath: null,
      vocoderPath: null,
      ruleFsts: [],
      speakerId: 0,
      speed: 1,
      numThreads: 1,
      device: "cpu",
      outputDeviceId: null,
    },
    updatedAt: "2026-08-22T00:00:00Z",
    ...overrides,
  };
}

function status(overrides: Partial<LocalMediaRuntimeStatus> = {}): LocalMediaRuntimeStatus {
  return {
    nativeAvailable: true,
    platformSupport: "supported",
    enabled: true,
    profileRevision: 4,
    pathClassifications: [],
    engines: [
      {
        engine: "ocr",
        readiness: { state: "ready" },
        profileRevision: 4,
        workerState: "idle",
        installedVersion: "3.0.0",
        modelIdentity: "PP-OCRv5",
        deviceSummary: "cpu",
        lastCheckedAt: "2026-08-22T01:02:03Z",
      },
      {
        engine: "stt",
        readiness: { state: "unavailable", code: "MODEL_NOT_FOUND", field: "modelDirectory" },
        profileRevision: 4,
        workerState: "quarantined",
        installedVersion: null,
        modelIdentity: null,
        deviceSummary: null,
        lastCheckedAt: null,
      },
      {
        engine: "tts",
        readiness: { state: "unconfigured" },
        profileRevision: 4,
        workerState: "stopped",
        installedVersion: null,
        modelIdentity: null,
        deviceSummary: null,
        lastCheckedAt: null,
      },
    ],
    ...overrides,
  };
}

function discovery(overrides: Partial<PythonEnvironmentDiscovery> = {}): PythonEnvironmentDiscovery {
  return {
    availability: "available",
    reasonCode: null,
    candidates: [
      {
        executablePath: "/opt/python-3.12/bin/python",
        version: { major: 3, minor: 12, patch: 4 },
        compatibility: "compatible",
        reasonCode: null,
        source: "path",
      },
    ],
    ...overrides,
  };
}

describe("buildLocalMediaDiagnosticFields", () => {
  it("disambiguates same-named fields across engines with an engine-prefixed label", () => {
    const fields = buildLocalMediaDiagnosticFields(profile(), status(), discovery(), false, t);
    const byLabel = new Map(fields.map((field) => [field.label, field.value]));

    // OCR and STT both have a "Language" field; OCR and STT and TTS all have a "Compute device"
    // field. Without the engine prefix these would collide into indistinguishable lines.
    expect(byLabel.get(engineLabel("localMedia.settings.ocr.shortTitle", "localMedia.settings.field.language"))).toBe("ch");
    expect(byLabel.get(engineLabel("localMedia.settings.stt.shortTitle", "localMedia.settings.field.language"))).toBe("auto");
    expect(byLabel.get(engineLabel("localMedia.settings.ocr.shortTitle", "localMedia.settings.field.device"))).toBe("auto");
    expect(byLabel.get(engineLabel("localMedia.settings.tts.shortTitle", "localMedia.settings.field.device"))).toBe("cpu");
  });

  it("reports readiness, worker state, and probe metadata as raw stable values, matching what the engine card already renders", () => {
    const fields = buildLocalMediaDiagnosticFields(profile(), status(), discovery(), false, t);
    const byLabel = new Map(fields.map((field) => [field.label, field.value]));
    const ocr = "localMedia.settings.ocr.shortTitle";

    expect(byLabel.get(engineLabel(ocr, "localMedia.diagnostics.field.readiness"))).toBe("ready");
    expect(byLabel.get(engineLabel(ocr, "localMedia.diagnostics.field.workerState"))).toBe("idle");
    expect(byLabel.get(engineLabel(ocr, "localMedia.settings.meta.version"))).toBe("3.0.0");
    expect(byLabel.get(engineLabel(ocr, "localMedia.settings.meta.model"))).toBe("PP-OCRv5");
    expect(byLabel.get(engineLabel(ocr, "localMedia.settings.meta.device"))).toBe("cpu");
    // Raw ISO, not locale-formatted -- a pasted diagnostic is more useful precise and
    // timezone-unambiguous than locale-pretty (same choice `cli-diagnostic-summary.ts` made).
    expect(byLabel.get(engineLabel(ocr, "localMedia.settings.meta.checkedAt"))).toBe("2026-08-22T01:02:03Z");
  });

  it("marks the readiness code and field only for the engine the failure actually named", () => {
    const fields = buildLocalMediaDiagnosticFields(profile(), status(), discovery(), false, t);
    const byLabel = new Map(fields.map((field) => [field.label, field.value]));
    const ocr = "localMedia.settings.ocr.shortTitle";
    const stt = "localMedia.settings.stt.shortTitle";

    expect(byLabel.get(engineLabel(stt, "localMedia.diagnostics.field.readiness"))).toBe("unavailable");
    expect(byLabel.get(engineLabel(stt, "localMedia.diagnostics.field.readinessCode"))).toBe("MODEL_NOT_FOUND");
    expect(byLabel.get(engineLabel(stt, "localMedia.diagnostics.field.readinessField"))).toBe("modelDirectory");
    // OCR is ready, so it must not carry a leftover/guessed code or field.
    expect(byLabel.get(engineLabel(ocr, "localMedia.diagnostics.field.readinessCode"))).toBeNull();
    expect(byLabel.get(engineLabel(ocr, "localMedia.diagnostics.field.readinessField"))).toBeNull();
  });

  it("collapses an unconfigured required path or interpreter to unavailable rather than showing a blank value", () => {
    // The default fixture's STT/TTS engines are disabled and unconfigured, exactly the shape
    // `pythonExecutable`/`modelDirectory`/`modelPath`/`tokensPath` have before a user fills them
    // in: typed as plain `string`, but empty.
    const fields = buildLocalMediaDiagnosticFields(profile(), status(), discovery(), false, t);
    const byLabel = new Map(fields.map((field) => [field.label, field.value]));
    const stt = "localMedia.settings.stt.shortTitle";
    const tts = "localMedia.settings.tts.shortTitle";

    expect(byLabel.get(engineLabel(stt, "localMedia.settings.field.pythonExecutable"))).toBeNull();
    expect(byLabel.get(engineLabel(stt, "localMedia.settings.field.modelDirectory"))).toBeNull();
    expect(byLabel.get(engineLabel(tts, "localMedia.settings.field.modelPath"))).toBeNull();
    expect(byLabel.get(engineLabel(tts, "localMedia.settings.field.tokensPath"))).toBeNull();
  });

  it("joins multiple configured rule FSTs into one field rather than dropping all but one", () => {
    const fields = buildLocalMediaDiagnosticFields(
      profile({ tts: { ...profile().tts, ruleFsts: ["/rules/a.fst", "/rules/b.fst"] } }),
      status(),
      discovery(),
      false,
      t,
    );
    const byLabel = new Map(fields.map((field) => [field.label, field.value]));

    expect(byLabel.get(engineLabel("localMedia.settings.tts.shortTitle", "localMedia.settings.field.ruleFsts"))).toBe(
      "/rules/a.fst, /rules/b.fst",
    );
  });

  it("reports Python discovery availability, reason, compatible count, and detected paths", () => {
    const fields = buildLocalMediaDiagnosticFields(profile(), status(), discovery(), false, t);
    const byLabel = new Map(fields.map((field) => [field.label, field.value]));

    expect(byLabel.get(t("localMedia.diagnostics.field.pythonAvailability"))).toBe("available");
    expect(byLabel.get(t("localMedia.diagnostics.field.pythonReasonCode"))).toBeNull();
    expect(byLabel.get(t("localMedia.diagnostics.field.pythonCompatibleCount"))).toBe("1");
    expect(byLabel.get(t("localMedia.diagnostics.field.pythonCandidatePaths"))).toBe("/opt/python-3.12/bin/python");
  });

  it("reports a reliably-known zero compatible count distinctly from discovery never having run", () => {
    const zero = buildLocalMediaDiagnosticFields(profile(), status(), discovery({ candidates: [] }), false, t);
    const neverRun = buildLocalMediaDiagnosticFields(profile(), status(), null, false, t);
    const byLabelZero = new Map(zero.map((field) => [field.label, field.value]));
    const byLabelNeverRun = new Map(neverRun.map((field) => [field.label, field.value]));

    expect(byLabelZero.get(t("localMedia.diagnostics.field.pythonCompatibleCount"))).toBe("0");
    expect(byLabelNeverRun.get(t("localMedia.diagnostics.field.pythonCompatibleCount"))).toBeNull();
  });

  it("reports the master switch and unsaved-changes state as the page's own top-level fields", () => {
    const dirtyFields = buildLocalMediaDiagnosticFields(profile({ enabled: false }), status(), discovery(), true, t);
    const byLabel = new Map(dirtyFields.map((field) => [field.label, field.value]));

    expect(byLabel.get(t("localMedia.settings.overview.master"))).toBe("false");
    expect(byLabel.get(t("localMedia.settings.overview.changes"))).toBe("true");
  });

  it("marks status-derived fields unavailable rather than omitting or inventing them before the first status load", () => {
    const fields = buildLocalMediaDiagnosticFields(profile(), null, discovery(), false, t);
    const byLabel = new Map(fields.map((field) => [field.label, field.value]));
    const ocr = "localMedia.settings.ocr.shortTitle";

    expect(byLabel.get(t("localMedia.diagnostics.field.nativeAvailable"))).toBeNull();
    expect(byLabel.get(t("localMedia.diagnostics.field.platformSupport"))).toBeNull();
    expect(byLabel.get(engineLabel(ocr, "localMedia.diagnostics.field.readiness"))).toBeNull();
    expect(byLabel.get(engineLabel(ocr, "localMedia.settings.meta.version"))).toBeNull();
    // The profile itself is unaffected by a missing status -- it is still the page's own draft.
    expect(byLabel.get(engineLabel(ocr, "localMedia.settings.field.pythonExecutable"))).toBe("/opt/ocr/bin/python");
  });

  it("never carries anything beyond the bounded fields this profile and status type can hold", () => {
    // Every field's value traces back to a path, a device id, a version string, a backend-pinned
    // enum/reason-code union, a plain number/boolean rendered as text, or a raw timestamp -- there
    // is no free-text field on this page's profile or status for this test to accidentally miss
    // redacting, the same structural guarantee `cli-diagnostic-summary.test.ts` proves for CLI.
    const fields = buildLocalMediaDiagnosticFields(profile(), status(), discovery(), true, t);
    expect(fields.length).toBeGreaterThan(60);
    expect(
      fields.every((field) => typeof field.label === "string" && (field.value === null || typeof field.value === "string")),
    ).toBe(true);
  });
});
