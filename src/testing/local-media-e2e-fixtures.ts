import type {
  LocalMediaEngine,
  LocalMediaErrorCode,
  LocalMediaOperationResult,
  LocalMediaProfile,
  LocalMediaRuntimeStatus,
} from "../types/local-media";

/**
 * Fixture data and mutable state for the E2E local-media fake.
 *
 * Split from the service itself only for file size; nothing here is reachable from a build that
 * does not set `VITE_LOCAL_MEDIA_FAKE`.
 */
export type OcrOutcome = "success" | "no-text" | "failure" | "picker-cancelled" | "staging-failure";
export type SttOutcome = "success" | "no-speech" | "failure" | "start-failure";
export type TtsOutcome = "success" | "failure" | "start-failure";

export interface Pending {
  remaining: number;
  settle: () => LocalMediaOperationResult;
  fail?: LocalMediaErrorCode;
}

export interface FakeState {
  nativeAvailable: boolean;
  ready: Record<LocalMediaEngine, boolean>;
  unavailable: Partial<Record<LocalMediaEngine, LocalMediaErrorCode>>;
  delay: number;
  staleScope: boolean;
  ocr: { outcome: OcrOutcome; text: string };
  stt: { outcome: SttOutcome; text: string };
  tts: { outcome: TtsOutcome };
  calls: Record<string, number>;
  pending: Map<string, Pending>;
  expired: Set<string>;
}

export function initialFakeState(): FakeState {
  return {
    nativeAvailable: true,
    ready: { ocr: true, stt: true, tts: true },
    unavailable: {},
    delay: 0,
    staleScope: false,
    ocr: { outcome: "success", text: "fixture recognized line one\nfixture recognized line two" },
    stt: { outcome: "success", text: "fixture transcript" },
    tts: { outcome: "success" },
    calls: {},
    pending: new Map(),
    expired: new Set(),
  };
}

/** A fully configured profile, so the settings page renders its populated layout. */
export function fixtureProfile(): LocalMediaProfile {
  const shared = { enabled: true, pythonExecutable: "/fixture/python" };
  return {
    profileId: "default",
    revision: 4,
    enabled: true,
    ocr: {
      ...shared,
      cpuAcceleration: "library-default",
      paddleXConfigPath: null,
      textDetectionModelDir: "/fixture/det",
      textRecognitionModelDir: "/fixture/rec",
      textLineOrientationModelDir: null,
      language: "ch",
      device: "cpu",
      maxPdfPages: 20,
    },
    stt: {
      ...shared,
      modelDirectory: "/fixture/whisper",
      device: "cpu",
      computeType: "int8",
      language: "auto",
      vadFilter: true,
      beamSize: 5,
      microphoneDeviceId: null,
      maxRecordingSeconds: 120,
    },
    tts: {
      ...shared,
      modelKind: "vits",
      modelPath: "/fixture/voice.onnx",
      tokensPath: "/fixture/tokens.txt",
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
  };
}

export function fixtureStatus(state: FakeState): LocalMediaRuntimeStatus {
  return {
    nativeAvailable: state.nativeAvailable,
    platformSupport: state.nativeAvailable ? "supported" : "unsupported",
    enabled: true,
    profileRevision: 4,
    pathClassifications: [],
    engines: (["ocr", "stt", "tts"] as const).map((engine) => ({
      engine,
      readiness: state.unavailable[engine]
        ? { state: "unavailable" as const, code: state.unavailable[engine] as LocalMediaErrorCode }
        : state.ready[engine]
          ? { state: "ready" as const }
          : { state: "unconfigured" as const },
      profileRevision: 4,
      workerState: state.ready[engine] ? ("idle" as const) : ("stopped" as const),
      installedVersion: state.ready[engine] ? "fixture-1.0.0" : null,
      modelIdentity: state.ready[engine] ? "fixture-model" : null,
      deviceSummary: state.ready[engine] ? "cpu" : null,
      lastCheckedAt: "2026-08-22T00:00:00Z",
    })),
  };
}

export function fixtureOcrResult(text: string): LocalMediaOperationResult {
  return {
    kind: "ocr",
    result: {
      source: { displayName: "fixture-invoice.png", mediaType: "image", pageCount: 1 },
      plainText: text,
      pages: [{ pageNumber: 1, text, lineCount: text ? text.split("\n").length : 0, lines: [] }],
      warnings: [],
      provenance: {
        engine: "paddleocr",
        engineVersion: "fixture-1.0.0",
        profileRevision: 4,
        language: "ch",
        modelIdentity: "fixture-model",
      },
      characterCount: text.length,
      truncated: false,
    },
  };
}

export function fixtureTranscript(text: string): LocalMediaOperationResult {
  return {
    kind: "stt",
    result: {
      text,
      detectedLanguage: "en",
      languageProbability: 0.99,
      durationMs: 1200,
      limitReached: false,
      provenance: {
        engine: "faster-whisper",
        engineVersion: "fixture-1.0.0",
        profileRevision: 4,
        device: "cpu",
      },
    },
  };
}

export function fixturePlayback(): LocalMediaOperationResult {
  return {
    kind: "tts",
    result: {
      playbackId: "fixture-playback",
      sampleRate: 22_050,
      durationMs: 900,
      deviceSummary: "Fixture Speaker",
    },
  };
}
