/**
 * Local media DTOs.
 *
 * These mirror the native `local_media` contract exactly. Every user-visible string the runtime
 * produces is a stable code or a locale key -- never prose -- because the native side has no
 * language and must not guess one.
 */

import type { LocalMediaErrorCode } from "./local-media-error-codes";

export const localMediaEngines = ["ocr", "stt", "tts"] as const;
export type LocalMediaEngine = (typeof localMediaEngines)[number];

export {
  localMediaErrorCodes,
  type LocalMediaErrorCode,
} from "./local-media-error-codes";

export type LocalMediaDevice = "auto" | "cpu" | "cuda";
export type WhisperComputeType = "auto" | "int8" | "float16" | "int8_float16" | "float32";
export type TtsModelKind = "vits" | "piper" | "kokoro" | "matcha";

/**
 * Whether the OCR worker asks PaddleOCR to use its CPU acceleration backend.
 *
 * Three values rather than a boolean: `library-default` passes no argument at all, which is not the
 * same statement as choosing `enabled`. It exists because paddlepaddle's oneDNN executor cannot run
 * every graph, and without a field an affected user has no recovery path.
 */
export type OcrCpuAcceleration = "library-default" | "enabled" | "disabled";

export interface PaddleOcrProfile {
  enabled: boolean;
  pythonExecutable: string;
  cpuAcceleration: OcrCpuAcceleration;
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

export type {
  PythonCompatibility,
  PythonDiscoveryAvailability,
  PythonDiscoveryReason,
  PythonDiscoverySource,
  PythonEnvironmentCandidate,
  PythonEnvironmentDiscovery,
  PythonVersion,
} from "./local-media-python";

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
  // `field` is present only when the engine itself named one. With several paths configured,
  // blaming the first in a list would send the user to edit a field that is fine.
  | { state: "unavailable"; code: LocalMediaErrorCode; field?: string }
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

/**
 * One model-related field's path, described by shape rather than by content.
 *
 * Never carries the path. `containsNonAscii` is a description and not a verdict: faster-whisper
 * reads non-ASCII paths, so only a failed canary makes one an error.
 */
export interface PathClassification {
  engine: LocalMediaEngine;
  field: string;
  configured: boolean;
  containsSpaces: boolean;
  containsNonAscii: boolean;
}

export interface LocalMediaRuntimeStatus {
  nativeAvailable: boolean;
  platformSupport: PlatformSupport;
  enabled: boolean;
  profileRevision: number;
  engines: EngineStatus[];
  pathClassifications: PathClassification[];
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
