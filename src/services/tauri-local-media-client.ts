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
 * Which file the picker returns.
 *
 * A desktop test build asks the native fixture for a repository-controlled image instead of
 * opening a dialog no headless runner can answer. Only the choice is replaced: the caller still
 * hands the path to the real `stage_local_media_ocr_source`, so sniffing, the size, page and pixel
 * ceilings, staging, the one-time claim and cleanup are unchanged.
 *
 * `FIXTURE_OCR_SOURCE_UNAVAILABLE` means fixtures were never activated -- the ordinary Desktop
 * Smoke layer runs this same build -- so the real dialog is correct there. When fixtures *are*
 * active the native side has already verified the file at startup and refused to boot without it,
 * which is where fail-closed lives.
 */
async function chooseOcrSource(): Promise<string | null> {
  if (import.meta.env.VITE_DESKTOP_E2E === "1") {
    try {
      return await invoke<string>("fixture_local_media_ocr_source");
    } catch {
      // Fixtures are off in this run; fall through to the real picker.
    }
  }
  const selected = await open({ multiple: false, filters: OCR_FILE_FILTERS });
  return typeof selected === "string" ? selected : null;
}

export const tauriLocalMediaClient: LocalMediaService = {
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
