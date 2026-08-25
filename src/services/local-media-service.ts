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
 * The composer and the settings page reach local OCR, speech recognition, and speech synthesis
 * only through this boundary. No component opens a dialog, spawns a process, or touches a device;
 * `selectAndStageOcrSource` in particular hides the native picker so a host path never enters
 * React state.
 */
export interface LocalMediaService {
  /** False in Web mode. Controls stay visible and disabled rather than disappearing. */
  isAvailable(): Promise<boolean>;

  getProfile(): Promise<LocalMediaProfile>;
  saveProfile(input: {
    profile: LocalMediaProfile;
    expectedRevision: number;
  }): Promise<LocalMediaProfile>;
  /**
   * Field-level validation for the settings form.
   *
   * Separate from `saveProfile` because a save failure carries only a stable code; this is what
   * tells the page which input to mark.
   */
  validateProfile(profile: LocalMediaProfile): Promise<ProfileFieldIssue[]>;

  getStatus(): Promise<LocalMediaRuntimeStatus>;
  listAudioDevices(): Promise<AudioDeviceCatalog>;
  probeEngine(engine: LocalMediaEngine): Promise<LocalMediaOperationHandle>;

  /**
   * Native picker for a profile path field. Resolves to `null` when the user cancels.
   *
   * The settings page does hold real paths -- that is what it configures -- but it must still not
   * import a Tauri module to obtain one, so the dialog lives behind this boundary like every other
   * native affordance.
   */
  selectProfilePath(input: { kind: "file" | "directory" }): Promise<string | null>;

  /** Opens the native picker. Resolves to `null` when the user cancels. */
  selectAndStageOcrSource(): Promise<StagedOcrSource | null>;
  /** Release a staged source the user picked and then abandoned. */
  discardStagedOcrSource(stagedInputId: string): Promise<void>;
  startOcr(input: {
    stagedInputId: string;
    composerScopeId: string;
  }): Promise<LocalMediaOperationHandle>;

  startRecording(input: { composerScopeId: string }): Promise<RecordingHandle>;
  stopRecordingAndTranscribe(input: {
    recordingId: string;
    composerScopeId: string;
  }): Promise<LocalMediaOperationHandle>;
  cancelRecording(input: { recordingId: string; composerScopeId: string }): Promise<void>;

  startTts(input: { text: string; composerScopeId: string }): Promise<LocalMediaOperationHandle>;
  stopPlayback(input: { playbackId?: string }): Promise<void>;

  cancelOperation(operationId: string): Promise<void>;
  /** `null` while the operation is still running. */
  getOperationResult(operationId: string): Promise<LocalMediaOperationResult | null>;
}
