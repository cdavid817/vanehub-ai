import { invoke } from "@tauri-apps/api/core";
import { open } from "@tauri-apps/plugin-dialog";

import type { LocalMediaService } from "./local-media-service";
import type {
  AudioDeviceCatalog,
  LocalMediaEngine,
  LocalMediaOperationHandle,
  LocalMediaOperationResult,
  LocalMediaProfile,
  LocalMediaRuntimeStatus,
  ProfileFieldIssue,
  RecordingHandle,
  StagedOcrSource,
} from "../types/local-media";
import { normalizePythonEnvironmentDiscovery } from "../types/local-media-python";
import { tauriScreenshotClient } from "../region-capture/tauri-screenshot-client";

/**
 * Formats the native picker offers.
 *
 * Deliberately narrower than what a file dialog could show: admission sniffs content and refuses
 * anything else, so offering TIFF here would only produce a rejection after the user chose it.
 */
const OCR_FILE_FILTERS = [
  { name: "Images and PDF", extensions: ["png", "jpg", "jpeg", "bmp", "pdf"] },
];

/**
 * Stable code the fixture command answers when the fixture runtime was never activated.
 *
 * The ordinary Desktop Smoke layer runs this same `desktop-e2e` artifact without the runtime
 * switch, so this one code -- and only this one -- means "there is no fixture here, use the real
 * dialog".
 */
const FIXTURE_UNAVAILABLE = "FIXTURE_OCR_SOURCE_UNAVAILABLE";

/** The trailing token of a command rejection is the stable code; anything else has none. */
function stableCodeOf(error: unknown): string | null {
  const raw =
    typeof error === "string" ? error : error instanceof Error ? error.message : String(error ?? "");
  const candidate = raw.trim().split(/\s+/u).pop() ?? "";
  return /^[A-Z][A-Z0-9_]*$/u.test(candidate) ? candidate : null;
}

/**
 * Which file the picker returns.
 *
 * A desktop test build asks the native fixture for a repository-controlled image instead of
 * opening a dialog no headless runner can answer. Only the choice is replaced: the caller still
 * hands the path to the real `stage_local_media_ocr_source`, so sniffing, the size, page and pixel
 * ceilings, staging, the one-time claim and cleanup are unchanged.
 *
 * Every other failure is rethrown rather than swallowed. A registry miss, an IPC fault, a path
 * encoding problem, or an internal error once fixtures *are* active are all real defects, and
 * quietly opening a dialog instead would turn each of them into a test that hangs waiting for a
 * human. Fail-closed for the activated case lives natively: the app refuses to boot without a
 * readable fixture source.
 */
async function chooseOcrSource(): Promise<string | null> {
  if (import.meta.env.VITE_DESKTOP_E2E === "1") {
    try {
      return await invoke<string>("fixture_local_media_ocr_source");
    } catch (error) {
      if (stableCodeOf(error) !== FIXTURE_UNAVAILABLE) throw error;
    }
  }
  const selected = await open({ multiple: false, filters: OCR_FILE_FILTERS });
  return typeof selected === "string" ? selected : null;
}

export const tauriLocalMediaClient: LocalMediaService = {
  ...tauriScreenshotClient,
  async isAvailable() {
    return true;
  },
  async getProfile() {
    return invoke<LocalMediaProfile>("get_local_media_profile");
  },
  async saveProfile(input) {
    return invoke<LocalMediaProfile>("save_local_media_profile", { request: input });
  },
  async validateProfile(profile) {
    return invoke<ProfileFieldIssue[]>("validate_local_media_profile", {
      request: { profile },
    });
  },
  async getStatus() {
    return invoke<LocalMediaRuntimeStatus>("get_local_media_status");
  },
  async listAudioDevices() {
    return invoke<AudioDeviceCatalog>("list_local_media_audio_devices");
  },
  async discoverPythonEnvironments() {
    return normalizePythonEnvironmentDiscovery(
      await invoke<unknown>("discover_local_media_python_environments"),
    );
  },
  async probeEngine(engine: LocalMediaEngine) {
    return invoke<LocalMediaOperationHandle>("start_local_media_probe", {
      request: { engine },
    });
  },
  async selectProfilePath(input) {
    const selected = await open({ directory: input.kind === "directory", multiple: false });
    return typeof selected === "string" ? selected : null;
  },
  /**
   * The picked path is used once, here, and is never returned to the caller.
   *
   * That is the whole reason staging is a separate step: the renderer learns an opaque id and a
   * display name, and the Python worker only ever sees the copy the host made.
   */
  async selectAndStageOcrSource() {
    const selected = await chooseOcrSource();
    if (typeof selected !== "string") {
      return null;
    }
    return invoke<StagedOcrSource>("stage_local_media_ocr_source", {
      request: { path: selected },
    });
  },
  async discardStagedOcrSource(stagedInputId) {
    await invoke("cleanup_local_media_staged_source", { request: { stagedInputId } });
  },
  async startOcr(input) {
    return invoke<LocalMediaOperationHandle>("start_local_media_ocr", { request: input });
  },
  async startRecording(input) {
    return invoke<RecordingHandle>("start_microphone_recording", { request: input });
  },
  async stopRecordingAndTranscribe(input) {
    return invoke<LocalMediaOperationHandle>("stop_recording_and_transcribe", {
      request: input,
    });
  },
  async cancelRecording(input) {
    await invoke("cancel_microphone_recording", { request: input });
  },
  async startTts(input) {
    return invoke<LocalMediaOperationHandle>("start_local_media_tts", { request: input });
  },
  async stopPlayback(input) {
    await invoke("stop_local_media_playback", {
      request: { playbackId: input.playbackId ?? null },
    });
  },
  async cancelOperation(operationId) {
    await invoke("cancel_local_media_operation", { request: { operationId } });
  },
  async getOperationResult(operationId) {
    return invoke<LocalMediaOperationResult | null>("get_local_media_operation_result", {
      request: { operationId },
    });
  },
};
