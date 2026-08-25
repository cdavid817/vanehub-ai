import type { LocalMediaService } from "../../services/local-media-service";
import type {
  LocalMediaErrorCode,
  OcrResult,
  StagedOcrSource,
} from "../../types/local-media";

/**
 * Plumbing every capability hook needs.
 *
 * Passed as one value rather than as six parameters because the three capability hooks are split
 * for file size, not because they are independent: they share the scope guard, the draft accessors,
 * and one failure surface.
 */
export interface LocalMediaComposerContext {
  /** `null` when no session owns the composer; every action is disabled. */
  composerScopeId: string | null;
  service: LocalMediaService;
  /** Reads the draft at the moment of use, never a snapshot taken when work started. */
  getDraft: () => string;
  setDraft: (value: string) => void;
  /** True when the operation was started by the composer that is still active. */
  ownsResult: (operationId: string) => boolean;
  remember: (operationId: string) => void;
  forget: (operationId: string) => void;
  reportFailure: (engine: LocalMediaFailure["engine"], error: unknown) => void;
  reportFailureCode: (engine: LocalMediaFailure["engine"], code: LocalMediaErrorCode) => void;
  reportOverflow: (text: string, engine: "ocr" | "stt") => void;
  clearFailure: () => void;
  /**
   * Return the caret to the end of the draft after text was appended.
   *
   * Called only on a successful append. It is a request, not a command: the implementation
   * declines when focus already sits on another substantive control, because pulling focus out of
   * a dialog or a menu the user opened while waiting is worse than leaving the caret where it is.
   */
  restoreCaret: () => void;
}

/** The composer's own input ceiling, shared by both append paths. */
export const MAX_DRAFT_CHARACTERS = 200_000;

/** What the OCR action is doing right now. */
export type OcrPhase = "idle" | "picking" | "running";

/**
 * The hold-to-talk state machine.
 *
 * `opening` is separate from `recording` because the device can take a noticeable moment and the
 * user must not see a recording indicator before capture has actually started -- if the microphone
 * is denied, nothing was ever recorded and the UI should never have claimed otherwise.
 */
export type MicrophonePhase =
  | "idle"
  | "opening"
  | "recording"
  | "finalizing"
  | "transcribing";

export type SpeechPhase = "idle" | "generating" | "playing";

/** Per-capability availability, so one unconfigured engine never disables the others. */
export interface LocalMediaAvailability {
  native: boolean;
  ocr: boolean;
  stt: boolean;
  tts: boolean;
}

/**
 * A failure the composer shows.
 *
 * `code` drives the localized message; there is no prose field, because the native side does not
 * produce any.
 */
export interface LocalMediaFailure {
  code: LocalMediaErrorCode;
  engine: "ocr" | "stt" | "tts";
}

/** The editable review shown before any OCR text touches the draft. */
export interface OcrReviewState {
  source: StagedOcrSource;
  result: OcrResult;
  text: string;
}

export interface LocalMediaComposerModel {
  availability: LocalMediaAvailability;
  ocrPhase: OcrPhase;
  microphonePhase: MicrophonePhase;
  speechPhase: SpeechPhase;
  /** Elapsed capture time in milliseconds; `0` unless recording. */
  recordingElapsedMs: number;
  /** True once capture stopped because it reached the configured ceiling. */
  recordingLimitReached: boolean;
  failure: LocalMediaFailure | null;
  review: OcrReviewState | null;
  /** Set when a completed result cannot be appended without exceeding a composer limit. */
  overflow: { text: string; engine: "ocr" | "stt" } | null;

  startOcr: () => void;
  updateReviewText: (text: string) => void;
  appendReviewText: () => void;
  cancelReview: () => void;

  microphone: {
    onPointerDown: (event: React.PointerEvent<HTMLButtonElement>) => void;
    onPointerUp: (event: React.PointerEvent<HTMLButtonElement>) => void;
    onPointerCancel: () => void;
    onLostPointerCapture: () => void;
    onKeyDown: (event: React.KeyboardEvent<HTMLButtonElement>) => void;
    onKeyUp: (event: React.KeyboardEvent<HTMLButtonElement>) => void;
    onClickCapture: (event: React.MouseEvent<HTMLButtonElement>) => void;
  };

  toggleSpeech: () => void;
  dismissFailure: () => void;
  dismissOverflow: () => void;
}
