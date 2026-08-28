import { vi } from "vitest";

import type { LocalMediaService } from "../../services/local-media-service";
import type {
  LocalMediaEngine,
  LocalMediaOperationResult,
  LocalMediaProfile,
  LocalMediaRuntimeStatus,
  OcrResult,
  StagedOcrSource,
  TranscriptionResult,
} from "../../types/local-media";

/**
 * A `LocalMediaService` whose operations settle only when a test says so.
 *
 * Real time is never involved. Every asynchronous edge this feature has -- a result arriving after
 * a session switch, a hold released while the previous transcript is still running -- is a question
 * about ordering, and a fake that resolved on a timer would only be able to ask it by accident.
 */
export interface LocalMediaDouble {
  service: LocalMediaService;
  /** Publish a result for the operation with this id; the next poll picks it up. */
  settle: (operationId: string, result: LocalMediaOperationResult) => void;
  /** Make the next poll for this operation reject with a stable code. */
  fail: (operationId: string, code: string) => void;
  calls: {
    cancelOperation: string[];
    /// Recorded as `recordingId@composerScopeId`.
    ///
    /// One array rather than two, because the pairing is the point: the native side refuses a
    /// cancel whose scope does not own the recording, so a test that only saw the id could not
    /// tell a correct release from one aimed at the session the user just switched to.
    cancelRecording: string[];
    discardStaged: string[];
    startOcr: string[];
    startRecording: string[];
    startTts: string[];
    /// Recorded as `recordingId@composerScopeId`, for the same reason.
    stopRecordingAndTranscribe: string[];
    stopPlayback: number;
  };
  setStaged: (staged: StagedOcrSource | null) => void;
  setStatus: (status: LocalMediaRuntimeStatus) => void;
  /**
   * Hold `startRecording` open until the test releases it.
   *
   * The window between pressing the microphone and the native side answering is where the hold
   * machine has a handle it cannot act on yet. A double that resolves immediately closes that
   * window, so every ordering question inside it can only be asked with an explicit deferral.
   */
  deferStartRecording: () => void;
  /** Answer the deferred `startRecording`. */
  resolveStartRecording: () => void;
  /** Reject the deferred `startRecording` with a stable code. */
  rejectStartRecording: (code: string) => void;
  /** Make the next `cancelRecording` reject, so the release path can be tested when it fails. */
  failNextCancelRecording: (code: string) => void;
}

export function readyStatus(
  ready: LocalMediaEngine[] = ["ocr", "stt", "tts"],
): LocalMediaRuntimeStatus {
  return {
    nativeAvailable: true,
    platformSupport: "supported",
    enabled: true,
    profileRevision: 3,
    pathClassifications: [],
    engines: (["ocr", "stt", "tts"] as const).map((engine) => ({
      engine,
      readiness: ready.includes(engine)
        ? { state: "ready" as const }
        : { state: "unconfigured" as const },
      profileRevision: 3,
      workerState: ready.includes(engine) ? ("idle" as const) : ("stopped" as const),
      installedVersion: ready.includes(engine) ? "1.2.3" : null,
      modelIdentity: null,
      deviceSummary: null,
      lastCheckedAt: null,
    })),
  };
}

export function ocrResult(plainText: string, overrides: Partial<OcrResult> = {}): OcrResult {
  return {
    source: { displayName: "invoice.png", mediaType: "image", pageCount: 1 },
    plainText,
    pages: [{ pageNumber: 1, text: plainText, lineCount: 1, lines: [] }],
    warnings: [],
    provenance: {
      engine: "paddleocr",
      engineVersion: "3.0.0",
      profileRevision: 3,
      language: "ch",
      modelIdentity: null,
    },
    characterCount: plainText.length,
    truncated: false,
    ...overrides,
  };
}

export function transcription(text: string, limitReached = false): TranscriptionResult {
  return {
    text,
    detectedLanguage: "en",
    languageProbability: 0.98,
    durationMs: 1_200,
    limitReached,
    provenance: {
      engine: "faster-whisper",
      engineVersion: "1.0.0",
      profileRevision: 3,
      device: "cpu",
    },
  };
}

export function stagedSource(overrides: Partial<StagedOcrSource> = {}): StagedOcrSource {
  return {
    stagedInputId: "staged-1",
    displayName: "invoice.png",
    mediaType: "image",
    byteLength: 2_048,
    ...overrides,
  };
}

export function createLocalMediaDouble(
  initialStatus: LocalMediaRuntimeStatus = readyStatus(),
): LocalMediaDouble {
  const results = new Map<string, LocalMediaOperationResult>();
  const failures = new Map<string, string>();
  let status = initialStatus;
  let staged: StagedOcrSource | null = stagedSource();
  let nextOperation = 0;

  const calls: LocalMediaDouble["calls"] = {
    cancelOperation: [],
    cancelRecording: [],
    discardStaged: [],
    startOcr: [],
    startRecording: [],
    startTts: [],
    stopRecordingAndTranscribe: [],
    stopPlayback: 0,
  };

  let deferStart = false;
  let pendingStart: { resolve: () => void; reject: (error: unknown) => void } | null = null;
  let recordingSequence = 0;
  let nextCancelFailure: string | null = null;

  const handle = (kind: LocalMediaOperationResult["kind"]) => {
    nextOperation += 1;
    return {
      operationId: `op-${nextOperation}`,
      kind: `local-media.${kind}` as const,
      acceptedAt: "2026-08-22T00:00:00Z",
    };
  };

  const service: LocalMediaService = {
    isAvailable: vi.fn(async () => true),
    getProfile: vi.fn(async () => ({}) as LocalMediaProfile),
    saveProfile: vi.fn(async () => ({}) as LocalMediaProfile),
    validateProfile: vi.fn(async () => []),
    getStatus: vi.fn(async () => status),
    listAudioDevices: vi.fn(async () => ({ inputs: [], outputs: [] })),
    discoverPythonEnvironments: vi.fn(async () => ({
      availability: "available" as const,
      reasonCode: null,
      candidates: [],
    })),
    probeEngine: vi.fn(async () => handle("probe")),
    selectProfilePath: vi.fn(async () => null),
    selectAndStageOcrSource: vi.fn(async () => staged),
    selectAndStageScreenshotRegion: vi.fn(async () => staged),
    commitScreenshotSelection: vi.fn(async () => undefined),
    cancelScreenshotSelection: vi.fn(async () => undefined),
    cancelActiveScreenshotSelection: vi.fn(async () => undefined),
    discardStagedOcrSource: vi.fn(async (id: string) => {
      calls.discardStaged.push(id);
    }),
    startOcr: vi.fn(async (input: { stagedInputId: string }) => {
      calls.startOcr.push(input.stagedInputId);
      return handle("ocr");
    }),
    startRecording: vi.fn(async (input: { composerScopeId: string }) => {
      calls.startRecording.push(input.composerScopeId);
      recordingSequence += 1;
      const started = {
        recordingId: `rec-${recordingSequence}`,
        startedAt: "2026-08-22T00:00:00Z",
        maxDurationMs: 120_000,
      };
      if (!deferStart) return started;
      return new Promise<typeof started>((resolve, reject) => {
        pendingStart = { resolve: () => resolve(started), reject };
      });
    }),
    stopRecordingAndTranscribe: vi.fn(
      async (input: { recordingId: string; composerScopeId: string }) => {
        calls.stopRecordingAndTranscribe.push(`${input.recordingId}@${input.composerScopeId}`);
        return handle("stt");
      },
    ),
    cancelRecording: vi.fn(async (input: { recordingId: string; composerScopeId: string }) => {
      calls.cancelRecording.push(`${input.recordingId}@${input.composerScopeId}`);
      const code = nextCancelFailure;
      nextCancelFailure = null;
      if (code) throw new Error(code);
    }),
    startTts: vi.fn(async (input: { text: string }) => {
      calls.startTts.push(input.text);
      return handle("tts");
    }),
    stopPlayback: vi.fn(async () => {
      calls.stopPlayback += 1;
    }),
    cancelOperation: vi.fn(async (operationId: string) => {
      calls.cancelOperation.push(operationId);
    }),
    getOperationResult: vi.fn(async (operationId: string) => {
      const code = failures.get(operationId);
      if (code) {
        failures.delete(operationId);
        throw new Error(code);
      }
      return results.get(operationId) ?? null;
    }),
  };

  return {
    service,
    settle: (operationId, result) => results.set(operationId, result),
    fail: (operationId, code) => failures.set(operationId, code),
    calls,
    setStaged: (next) => {
      staged = next;
    },
    setStatus: (next) => {
      status = next;
    },
    deferStartRecording: () => {
      deferStart = true;
    },
    resolveStartRecording: () => {
      const pending = pendingStart;
      pendingStart = null;
      pending?.resolve();
    },
    failNextCancelRecording: (code) => {
      nextCancelFailure = code;
    },
    rejectStartRecording: (code) => {
      const pending = pendingStart;
      pendingStart = null;
      pending?.reject(new Error(code));
    },
  };
}
