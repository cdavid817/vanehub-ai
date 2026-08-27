import { Camera, LoaderCircle, Mic, ScanText, Square, Volume2, X } from "lucide-react";
import { useTranslation } from "react-i18next";

import { localMediaMessageKey } from "../../session-workspace/local-media/local-media-errors";
import type { LocalMediaComposerModel } from "../../session-workspace/local-media/local-media-composer-types";
import { Button } from "../ui/button";
import { RecordingIndicator } from "./RecordingIndicator";

/**
 * The three compact media actions.
 *
 * Presentational by construction: it receives state and callbacks and never reaches a service,
 * polls an operation, or touches the session model. Every control is a fixed 32x32 square, and a
 * spinner replaces an icon inside that square rather than beside it -- the toolbar must not move
 * when a state changes, because it sits directly under the text the user is typing.
 */
const ACTION_CLASS = "h-8 w-8 shrink-0 p-0";

export function ComposerMediaActions({
  media,
  hasText,
}: {
  media: LocalMediaComposerModel;
  /** Whether there is anything for speech to read. */
  hasText: boolean;
}) {
  const { t } = useTranslation();
  const { availability, microphonePhase, ocrPhase, speechPhase } = media;

  const recording = microphonePhase === "recording" || microphonePhase === "opening";
  const speaking = speechPhase !== "idle";
  const unavailableReason = availability.native
    ? "localMedia.composer.unready"
    : "localMedia.composer.nativeOnly";

  return (
    <div className="flex items-center gap-1" data-testid="composer-media-actions">
      <Button
        aria-label={t("localMedia.composer.ocr")}
        className={ACTION_CLASS}
        data-testid="composer-media-ocr"
        disabled={!availability.ocr || ocrPhase !== "idle"}
        onClick={media.startOcr}
        title={availability.ocr ? t("localMedia.composer.ocrTitle") : t(unavailableReason)}
        type="button"
        variant="ghost"
      >
        {ocrPhase === "idle" ? (
          <ScanText aria-hidden="true" className="h-4 w-4" />
        ) : (
          <LoaderCircle aria-hidden="true" className="h-4 w-4 motion-safe:animate-spin" />
        )}
      </Button>

      <Button
        aria-label={t("localMedia.composer.screenshot")}
        className={ACTION_CLASS}
        data-testid="composer-media-screenshot"
        disabled={!availability.screenshot || ocrPhase !== "idle"}
        onClick={media.startScreenshot}
        title={availability.screenshot
          ? t("localMedia.composer.screenshotTitle")
          : t(unavailableReason)}
        type="button"
        variant="ghost"
      >
        {ocrPhase === "picking" ? (
          <LoaderCircle aria-hidden="true" className="h-4 w-4 motion-safe:animate-spin" />
        ) : (
          <Camera aria-hidden="true" className="h-4 w-4" />
        )}
      </Button>

      <Button
        aria-label={t("localMedia.composer.hold")}
        aria-pressed={recording}
        className={ACTION_CLASS}
        data-testid="composer-media-microphone"
        disabled={!availability.stt || microphonePhase === "transcribing"}
        title={availability.stt ? t("localMedia.composer.holdTitle") : t(unavailableReason)}
        type="button"
        variant={recording ? "outline" : "ghost"}
        {...media.microphone}
      >
        {microphonePhase === "finalizing" || microphonePhase === "transcribing" ? (
          <LoaderCircle aria-hidden="true" className="h-4 w-4 motion-safe:animate-spin" />
        ) : (
          <Mic
            aria-hidden="true"
            className={
              recording ? "h-4 w-4 text-danger motion-safe:animate-pulse" : "h-4 w-4"
            }
          />
        )}
      </Button>

      <Button
        aria-label={speaking ? t("localMedia.composer.stopSpeech") : t("localMedia.composer.speak")}
        aria-pressed={speaking}
        className={ACTION_CLASS}
        data-testid="composer-media-speak"
        disabled={!speaking && (!availability.tts || !hasText)}
        onClick={media.toggleSpeech}
        title={availability.tts ? t("localMedia.composer.speakTitle") : t(unavailableReason)}
        type="button"
        variant={speaking ? "outline" : "ghost"}
      >
        {speechPhase === "generating" ? (
          <LoaderCircle aria-hidden="true" className="h-4 w-4 motion-safe:animate-spin" />
        ) : speechPhase === "playing" ? (
          <Square aria-hidden="true" className="h-4 w-4" />
        ) : (
          <Volume2 aria-hidden="true" className="h-4 w-4" />
        )}
      </Button>

      <RecordingIndicator
        elapsedMs={media.recordingElapsedMs}
        limitReached={media.recordingLimitReached}
        phase={microphonePhase}
      />

      {/* Inline and dismissible rather than a toast: the failure belongs to the control the user
          just pressed, and a toast would outlive the composer it refers to after a session
          switch. Truncated with the full text in `title`, because this sits in a toolbar that
          must not change height. */}
      {media.failure ? (
        <span
          className="flex max-w-64 items-center gap-1 text-[11px] text-danger"
          data-testid="composer-media-failure"
          role="alert"
        >
          <span className="truncate" title={t(localMediaMessageKey(media.failure.code))}>
            {t(localMediaMessageKey(media.failure.code))}
          </span>
          <button
            aria-label={t("localMedia.composer.dismissFailure")}
            className="shrink-0 rounded-sm p-0.5 hover:bg-muted focus-visible:outline-hidden focus-visible:ring-2 focus-visible:ring-ring"
            onClick={media.dismissFailure}
            type="button"
          >
            <X aria-hidden="true" className="h-3 w-3" />
          </button>
        </span>
      ) : null}
    </div>
  );
}
