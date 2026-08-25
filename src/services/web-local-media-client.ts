import type { LocalMediaService } from "./local-media-service";
import type { LocalMediaProfile, LocalMediaRuntimeStatus } from "../types/local-media";

/**
 * Web/mock behaviour, and it is deliberately inert.
 *
 * Every other mock in this application simulates a plausible success so the browser build stays
 * explorable. This one must not: OCR, a microphone, and a speech engine are native capabilities,
 * and a mock transcript would be indistinguishable from a real one to anyone evaluating whether
 * the feature works. Tests that need success states inject a fake through the service boundary
 * instead of relying on this.
 */
const NATIVE_ONLY = "LOCAL_MEDIA_NATIVE_ONLY";

function nativeOnly(): never {
  throw new Error(NATIVE_ONLY);
}

/** Disabled defaults, so the settings page renders its real layout with nothing configured. */
function disabledProfile(): LocalMediaProfile {
  return {
    profileId: "default",
    revision: 0,
    enabled: false,
    ocr: {
      enabled: false,
      pythonExecutable: "",
      cpuAcceleration: "library-default",
      paddleXConfigPath: null,
      textDetectionModelDir: null,
      textRecognitionModelDir: null,
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
    updatedAt: "1970-01-01T00:00:00Z",
  };
}

function unavailableStatus(): LocalMediaRuntimeStatus {
  return {
    nativeAvailable: false,
    platformSupport: "unsupported",
    enabled: false,
    profileRevision: 0,
    pathClassifications: [],
    engines: (["ocr", "stt", "tts"] as const).map((engine) => ({
      engine,
      readiness: { state: "unavailable", code: NATIVE_ONLY },
      profileRevision: 0,
      workerState: "stopped",
      installedVersion: null,
      modelIdentity: null,
      deviceSummary: null,
      lastCheckedAt: null,
    })),
  };
}

export const webLocalMediaClient: LocalMediaService = {
  async isAvailable() {
    return false;
  },

  async getProfile() {
    return disabledProfile();
  },

  async saveProfile() {
    return nativeOnly();
  },

  async validateProfile() {
    // Shape validation would be honest here, but reporting "no problems" for a profile that can
    // never run reads as encouragement. The page shows the native-only notice instead.
    return [];
  },

  async getStatus() {
    return unavailableStatus();
  },

  async listAudioDevices() {
    return { inputs: [], outputs: [] };
  },

  async probeEngine() {
    return nativeOnly();
  },

  async selectProfilePath() {
    // A browser cannot produce a host path, and prompting for one by hand would let the user save
    // a profile that is unreachable by definition.
    return null;
  },

  async selectAndStageOcrSource() {
    return nativeOnly();
  },

  async discardStagedOcrSource() {
    return nativeOnly();
  },

  async startOcr() {
    return nativeOnly();
  },

  async startRecording() {
    return nativeOnly();
  },

  async stopRecordingAndTranscribe() {
    return nativeOnly();
  },

  async cancelRecording() {
    return nativeOnly();
  },

  async startTts() {
    return nativeOnly();
  },

  async stopPlayback() {
    return nativeOnly();
  },

  async cancelOperation() {
    return nativeOnly();
  },

  async getOperationResult() {
    return null;
  },
};
