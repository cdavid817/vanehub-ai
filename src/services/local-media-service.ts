import type {
  AudioDeviceCatalog,
  LocalMediaEngine,
  LocalMediaOperationHandle,
  LocalMediaOperationResult,
  LocalMediaProfile,
  LocalMediaRuntimeStatus,
  ProfileFieldIssue,
  PythonEnvironmentDiscovery,
  RecordingHandle,
  StagedOcrSource,
} from "../types/local-media";
import type { ScreenshotService } from "../region-capture/screenshot-service-contract";

/** Local media reaches native pickers, processes, and devices only through this boundary. */
export interface LocalMediaService extends ScreenshotService {
  /** False in Web mode. Controls stay visible and disabled rather than disappearing. */
  isAvailable(): Promise<boolean>;

  getProfile(): Promise<LocalMediaProfile>;
  saveProfile(input: {
    profile: LocalMediaProfile;
    expectedRevision: number;
  }): Promise<LocalMediaProfile>;
  /** Field-level validation stays separate because save failures carry only stable codes. */
  validateProfile(profile: LocalMediaProfile): Promise<ProfileFieldIssue[]>;

  getStatus(): Promise<LocalMediaRuntimeStatus>;
  listAudioDevices(): Promise<AudioDeviceCatalog>;
  discoverPythonEnvironments(): Promise<PythonEnvironmentDiscovery>;
  probeEngine(engine: LocalMediaEngine): Promise<LocalMediaOperationHandle>;

  /** Native profile-path picker; `null` means the user cancelled. */
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
