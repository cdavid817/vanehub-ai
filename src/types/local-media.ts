/**
 * Local media DTOs.
 *
 * These mirror the native `local_media` contract exactly. Every user-visible string the runtime
 * produces is a stable code or a locale key -- never prose -- because the native side has no
 * language and must not guess one.
 */

export const localMediaEngines = ["ocr", "stt", "tts"] as const;
export type LocalMediaEngine = (typeof localMediaEngines)[number];

export const localMediaErrorCodes = [
  "LOCAL_MEDIA_NATIVE_ONLY",
  "LOCAL_MEDIA_DISABLED",
  "ENGINE_DISABLED",
  "ENGINE_UNCONFIGURED",
  "ENGINE_BUSY",
  "ENGINE_UNAVAILABLE",
  "PYTHON_NOT_FOUND",
  "PYTHON_EXECUTION_DENIED",
  "ENGINE_IMPORT_FAILED",
  "ENGINE_VERSION_UNSUPPORTED",
  "PROFILE_REVISION_CONFLICT",
  "MODEL_NOT_CONFIGURED",
  "MODEL_NOT_FOUND",
  "MODEL_INCOMPATIBLE",
  "MODEL_DOWNLOAD_BLOCKED",
  "DEVICE_CONFIGURATION_INVALID",
  "MIC_PERMISSION_DENIED",
  "MIC_DEVICE_UNAVAILABLE",
  "AUDIO_CAPTURE_START_FAILED",
  "AUDIO_CAPTURE_OVERRUN",
  "RECORDING_ALREADY_ACTIVE",
  "RECORDING_NOT_FOUND",
  "RECORDING_TOO_SHORT",
  "RECORDING_LIMIT_REACHED",
  "INPUT_NOT_FOUND",
  "INPUT_TOO_LARGE",
  "UNSUPPORTED_MEDIA_TYPE",
  "PDF_PAGE_LIMIT_EXCEEDED",
  "IMAGE_PIXEL_LIMIT_EXCEEDED",
  "NO_TEXT_DETECTED",
  "NO_SPEECH_DETECTED",
  "TTS_TEXT_TOO_LONG",
  "PLAYBACK_DEVICE_UNAVAILABLE",
  "WORKER_START_FAILED",
  "WORKER_CRASHED",
  "WORKER_PROTOCOL_ERROR",
  "OPERATION_CANCELLED",
  "OPERATION_RESULT_EXPIRED",
  "TEMP_STORAGE_FAILED",
  "TEMP_CLEANUP_FAILED",
] as const;
export type LocalMediaErrorCode = (typeof localMediaErrorCodes)[number];

export type LocalMediaDevice = "auto" | "cpu" | "cuda";
export type WhisperComputeType = "auto" | "int8" | "float16" | "int8_float16" | "float32";
export type TtsModelKind = "vits" | "piper" | "kokoro" | "matcha";

export interface PaddleOcrProfile {
  enabled: boolean;
  pythonExecutable: string;
  paddleXConfigPath: string | null;
  textDetectionModelDir: string | null;
  textRecognitionModelDir: string | null;
  textLineOrientationModelDir: string | null;
  language: string;
  device: LocalMediaDevice;
  maxPdfPages: number;
}

export interface FasterWhisperProfile {
  enabled: boolean;
  pythonExecutable: string;
  modelDirectory: string;
  device: LocalMediaDevice;
  computeType: WhisperComputeType;
  language: string;
  vadFilter: boolean;
  beamSize: number;
  microphoneDeviceId: string | null;
  maxRecordingSeconds: number;
}

export interface SherpaOnnxTtsProfile {
  enabled: boolean;
  pythonExecutable: string;
  modelKind: TtsModelKind;
  modelPath: string;
  tokensPath: string;
  lexiconPath: string | null;
  dataDir: string | null;
  dictDir: string | null;
  voicesPath: string | null;
  vocoderPath: string | null;
  ruleFsts: string[];
  speakerId: number;
  speed: number;
  numThreads: number;
  device: LocalMediaDevice;
  outputDeviceId: string | null;
}

export interface LocalMediaProfile {
  profileId: "default";
  revision: number;
  enabled: boolean;
  ocr: PaddleOcrProfile;
  stt: FasterWhisperProfile;
  tts: SherpaOnnxTtsProfile;
  updatedAt: string;
}

/** One rejected field, addressed to the input that produced it. */
export interface ProfileFieldIssue {
  engine: LocalMediaEngine | null;
  field: string;
  code: LocalMediaErrorCode;
  messageKey: string;
}

export type EngineReadiness =
  | { state: "disabled" }
  | { state: "unconfigured" }
  | { state: "checking" }
  | { state: "ready" }
  | { state: "unavailable"; code: LocalMediaErrorCode }
  | { state: "restartRequired" };

export type WorkerState =
  | "stopped"
  | "starting"
  | "idle"
  | "busy"
  | "restarting"
  | "quarantined";

export type PlatformSupport = "supported" | "experimental" | "unsupported";

export interface EngineStatus {
  engine: LocalMediaEngine;
  readiness: EngineReadiness;
  profileRevision: number;
  workerState: WorkerState;
  installedVersion: string | null;
  modelIdentity: string | null;
  deviceSummary: string | null;
  lastCheckedAt: string | null;
}

export interface LocalMediaRuntimeStatus {
  nativeAvailable: boolean;
  platformSupport: PlatformSupport;
  enabled: boolean;
  profileRevision: number;
  engines: EngineStatus[];
}

export interface AudioDevice {
  deviceId: string;
  label: string;
  isDefault: boolean;
}

export interface AudioDeviceCatalog {
  inputs: AudioDevice[];
  outputs: AudioDevice[];
}

export type LocalMediaOperationKind =
  | "local-media.probe"
  | "local-media.ocr"
  | "local-media.stt"
  | "local-media.tts";

export interface LocalMediaOperationHandle {
  operationId: string;
  kind: LocalMediaOperationKind;
  acceptedAt: string;
}

export interface RecordingHandle {
  recordingId: string;
  startedAt: string;
  maxDurationMs: number;
}

export type OcrMediaType = "image" | "pdf";

/** What the composer learns about a picked file: never a path. */
export interface StagedOcrSource {
  stagedInputId: string;
  displayName: string;
  mediaType: OcrMediaType;
  byteLength: number;
}

export interface OcrLine {
  text: string;
  confidence: number | null;
  polygon: Array<[number, number]> | null;
}

export interface OcrPage {
  pageNumber: number;
  text: string;
  lineCount: number;
  lines: OcrLine[];
}

export interface OcrWarning {
  code: string;
  messageKey: string;
  pageNumber: number | null;
}

export interface OcrResult {
  source: { displayName: string; mediaType: OcrMediaType; pageCount: number };
  plainText: string;
  pages: OcrPage[];
  warnings: OcrWarning[];
  provenance: {
    engine: string;
    engineVersion: string | null;
    profileRevision: number;
    language: string;
    modelIdentity: string | null;
  };
  characterCount: number;
  truncated: boolean;
}

export interface TranscriptionResult {
  text: string;
  detectedLanguage: string | null;
  languageProbability: number | null;
  durationMs: number | null;
  limitReached: boolean;
  provenance: {
    engine: string;
    engineVersion: string | null;
    profileRevision: number;
    device: string;
  };
}

export interface SpeechPlaybackResult {
  playbackId: string;
  sampleRate: number;
  durationMs: number;
  deviceSummary: string | null;
}

export type LocalMediaOperationResult =
  | { kind: "probe"; result: LocalMediaRuntimeStatus }
  | { kind: "ocr"; result: OcrResult }
  | { kind: "stt"; result: TranscriptionResult }
  | { kind: "tts"; result: SpeechPlaybackResult };
