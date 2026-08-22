import type { LocalMediaService } from "../services/local-media-service";
import type {
  LocalMediaEngine,
  LocalMediaErrorCode,
  LocalMediaOperationHandle,
  LocalMediaOperationResult,
} from "../types/local-media";
import {
  fixtureOcrResult,
  fixturePlayback,
  fixtureProfile,
  fixtureStatus,
  fixtureTranscript,
  initialFakeState,
  type OcrOutcome,
  type SttOutcome,
  type TtsOutcome,
} from "./local-media-e2e-fixtures";

/**
 * A scriptable stand-in for the native local-media service, reachable only from a build that sets
 * `VITE_LOCAL_MEDIA_FAKE`.
 *
 * It exists so browser E2E can drive success, emptiness, failure, cancellation, delay, and scope
 * races deterministically. The production Web adapter must never simulate any of that: a
 * fabricated transcript is indistinguishable from a real one to anyone judging whether the feature
 * works. Keeping that honesty and still getting UI coverage is only possible if the two live in
 * different builds.
 *
 * The control surface hangs off `window`, which is safe precisely because this module is absent
 * from any build without the flag. There is no production code to activate, so there is no
 * production switch to find -- no URL parameter, no storage key, no command, no settings toggle.
 * `local-media-fake-boundary.test.ts` asserts the built production bundle contains neither this
 * module's activation token nor the control name.
 */
const CONTROL_KEY = "__vanehubLocalMediaFake";

export interface LocalMediaFakeControl {
  reset(): void;
  setNativeAvailable(available: boolean): void;
  setEngineReady(engine: LocalMediaEngine, ready: boolean): void;
  setEngineUnavailable(engine: LocalMediaEngine, code: LocalMediaErrorCode): void;
  /** Number of `getOperationResult` polls an operation stays pending before it settles. */
  setOperationDelay(polls: number): void;
  /** Drop stored results so the next read reports the operation as expired. */
  expireResults(): void;
  /** Make the next result arrive for a scope the composer has already left. */
  setStaleScope(stale: boolean): void;
  scriptOcr(outcome: OcrOutcome, text?: string): void;
  scriptStt(outcome: SttOutcome, text?: string): void;
  scriptTts(outcome: TtsOutcome): void;
  /** The text the last `startTts` received, so selection-versus-draft is assertable. */
  lastTtsText(): string | null;
  calls(): Record<string, number>;
}

declare global {
  interface Window {
    [CONTROL_KEY]?: LocalMediaFakeControl;
  }
}

export function createDeterministicFakeLocalMediaService(): LocalMediaService {
  const state = initialFakeState();
  let nextId = 0;
  let lastTtsText: string | null = null;
  const count = (name: string) => {
    state.calls[name] = (state.calls[name] ?? 0) + 1;
  };
  const reject = (code: LocalMediaErrorCode): never => {
    throw new Error(code);
  };

  function accept(
    kind: LocalMediaOperationHandle["kind"],
    settle: () => LocalMediaOperationResult,
    failure?: LocalMediaErrorCode,
  ): LocalMediaOperationHandle {
    nextId += 1;
    const operationId = `fake-op-${nextId}`;
    state.pending.set(operationId, { remaining: state.delay, settle, fail: failure });
    return { operationId, kind, acceptedAt: "2026-08-22T00:00:00Z" };
  }

  const control: LocalMediaFakeControl = {
    reset: () => {
      Object.assign(state, initialFakeState());
      lastTtsText = null;
    },
    setNativeAvailable: (available) => {
      state.nativeAvailable = available;
    },
    setEngineReady: (engine, ready) => {
      state.ready[engine] = ready;
      delete state.unavailable[engine];
    },
    setEngineUnavailable: (engine, code) => {
      state.ready[engine] = false;
      state.unavailable[engine] = code;
    },
    setOperationDelay: (polls) => {
      state.delay = polls;
    },
    expireResults: () => {
      for (const key of state.pending.keys()) state.expired.add(key);
      state.pending.clear();
    },
    setStaleScope: (stale) => {
      state.staleScope = stale;
    },
    scriptOcr: (outcome, text) => {
      state.ocr = { outcome, text: text ?? state.ocr.text };
    },
    scriptStt: (outcome, text) => {
      state.stt = { outcome, text: text ?? state.stt.text };
    },
    scriptTts: (outcome) => {
      state.tts = { outcome };
    },
    lastTtsText: () => lastTtsText,
    calls: () => ({ ...state.calls }),
  };
  if (typeof window !== "undefined") window[CONTROL_KEY] = control;

  return {
    isAvailable: async () => state.nativeAvailable,
    getProfile: async () => fixtureProfile(),
    saveProfile: async (input) => ({ ...input.profile, revision: input.profile.revision + 1 }),
    validateProfile: async () => [],
    getStatus: async () => fixtureStatus(state),
    listAudioDevices: async () => ({
      inputs: [{ deviceId: "fixture-mic", label: "Fixture Microphone", isDefault: true }],
      outputs: [{ deviceId: "fixture-out", label: "Fixture Speaker", isDefault: true }],
    }),
    probeEngine: async () => {
      count("probeEngine");
      return accept("local-media.probe", () => ({ kind: "probe", result: fixtureStatus(state) }));
    },
    selectProfilePath: async () => "/fixture/selected",
    selectAndStageOcrSource: async () => {
      count("selectAndStageOcrSource");
      if (state.ocr.outcome === "picker-cancelled") return null;
      if (state.ocr.outcome === "staging-failure") reject("UNSUPPORTED_MEDIA_TYPE");
      return {
        stagedInputId: "fixture-staged",
        displayName: "fixture-invoice.png",
        mediaType: "image",
        byteLength: 4096,
      };
    },
    discardStagedOcrSource: async () => {
      count("discardStagedOcrSource");
    },
    startOcr: async () => {
      count("startOcr");
      const settle = () => fixtureOcrResult(state.ocr.outcome === "no-text" ? "" : state.ocr.text);
      return state.ocr.outcome === "failure"
        ? accept("local-media.ocr", settle, "WORKER_CRASHED")
        : accept("local-media.ocr", settle);
    },
    startRecording: async () => {
      count("startRecording");
      if (state.stt.outcome === "start-failure") reject("MIC_PERMISSION_DENIED");
      return {
        recordingId: "fixture-recording",
        startedAt: "2026-08-22T00:00:00Z",
        maxDurationMs: 120_000,
      };
    },
    stopRecordingAndTranscribe: async () => {
      count("stopRecordingAndTranscribe");
      const settle = () => fixtureTranscript(state.stt.text);
      if (state.stt.outcome === "failure") return accept("local-media.stt", settle, "WORKER_CRASHED");
      if (state.stt.outcome === "no-speech") {
        return accept("local-media.stt", settle, "NO_SPEECH_DETECTED");
      }
      return accept("local-media.stt", settle);
    },
    cancelRecording: async () => {
      count("cancelRecording");
    },
    startTts: async (input) => {
      count("startTts");
      lastTtsText = input.text;
      if (state.tts.outcome === "start-failure") reject("TTS_TEXT_TOO_LONG");
      return state.tts.outcome === "failure"
        ? accept("local-media.tts", fixturePlayback, "PLAYBACK_DEVICE_UNAVAILABLE")
        : accept("local-media.tts", fixturePlayback);
    },
    stopPlayback: async () => {
      count("stopPlayback");
    },
    cancelOperation: async (operationId) => {
      count("cancelOperation");
      const entry = state.pending.get(operationId);
      if (entry) entry.fail = "OPERATION_CANCELLED";
    },
    getOperationResult: async (operationId) => {
      if (state.expired.has(operationId)) reject("OPERATION_RESULT_EXPIRED");
      const entry = state.pending.get(operationId);
      if (!entry) return null;
      if (entry.remaining > 0) {
        entry.remaining -= 1;
        return null;
      }
      state.pending.delete(operationId);
      // A stale scope is expressed as a result that never arrives, which is what the real service
      // produces for a composer the user has left: the work finished, for nobody.
      if (state.staleScope) return null;
      if (entry.fail) reject(entry.fail);
      return entry.settle();
    },
  };
}
