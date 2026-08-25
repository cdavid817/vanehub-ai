import { useTranslation } from "react-i18next";

import type { MicrophonePhase } from "../../session-workspace/local-media/local-media-composer-types";

function formatElapsed(elapsedMs: number): string {
  const totalSeconds = Math.floor(elapsedMs / 1000);
  const minutes = Math.floor(totalSeconds / 60);
  const seconds = totalSeconds % 60;
  return `${String(minutes).padStart(2, "0")}:${String(seconds).padStart(2, "0")}`;
}

/**
 * Elapsed time and state for an active hold.
 *
 * The status text and the timer are separated deliberately. The timer changes every second and is
 * marked `aria-hidden`; a screen reader that announced it would produce one interruption per
 * second and drown out everything else. The status line beside it is a polite live region and only
 * changes when the phase does, which is the transition a non-sighted user actually needs.
 */
export function RecordingIndicator({
  elapsedMs,
  limitReached,
  phase,
}: {
  elapsedMs: number;
  limitReached: boolean;
  phase: MicrophonePhase;
}) {
  const { t } = useTranslation();
  if (phase === "idle") {
    return (
      <span aria-live="polite" className="sr-only" data-testid="composer-media-status">
        {limitReached ? t("localMedia.composer.limitReached") : ""}
      </span>
    );
  }

  const statusKey =
    phase === "opening"
      ? "localMedia.composer.opening"
      : phase === "recording"
        ? "localMedia.composer.recording"
        : phase === "finalizing"
          ? "localMedia.composer.finalizing"
          : "localMedia.composer.transcribing";

  return (
    <span
      className="flex items-center gap-1.5 text-[11px] text-muted-foreground"
      data-testid="composer-media-recording"
    >
      <span aria-live="polite" data-testid="composer-media-status">
        {t(statusKey)}
      </span>
      {phase === "recording" ? (
        <>
          <span
            aria-hidden="true"
            className="tabular-nums"
            data-testid="composer-media-elapsed"
          >
            {formatElapsed(elapsedMs)}
          </span>
          <span aria-hidden="true">{t("localMedia.composer.escapeHint")}</span>
        </>
      ) : null}
    </span>
  );
}
