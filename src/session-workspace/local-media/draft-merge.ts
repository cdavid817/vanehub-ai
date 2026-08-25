/**
 * How a local-media result joins the draft.
 *
 * Both helpers append and never replace. The draft the user is looking at when a result arrives is
 * not the one that was there when they started the recording, and overwriting it would discard
 * whatever they typed while the model was loading.
 */

/** Line endings only. Recognized punctuation and internal spacing are the engine's, not ours. */
function normalize(value: string): string {
  return value.replace(/\r\n?/gu, "\n").trim();
}

/**
 * Speech joins with a single space.
 *
 * A transcript continues the sentence the user was typing, so a space is the right separator --
 * and if the draft already ends in whitespace, adding another would show up as a double space in
 * the message they send.
 */
export function appendSpeechTranscript(current: string, transcript: string): string {
  const normalized = normalize(transcript);
  if (!normalized) return current;
  if (!current) return normalized;
  if (/\s$/u.test(current)) return `${current}${normalized}`;
  return `${current} ${normalized}`;
}

/**
 * OCR joins with a blank line.
 *
 * Recognized text is a block, not a continuation: it came from a separate document and reads as
 * its own paragraph. A draft already ending in a newline gets no extra separator, so a user who
 * positioned their own break keeps it.
 */
export function appendOcrText(current: string, text: string): string {
  const normalized = normalize(text);
  if (!normalized) return current;
  if (!current) return normalized;
  if (/\n$/u.test(current)) return `${current}${normalized}`;
  return `${current}\n\n${normalized}`;
}

/**
 * What TTS reads: the selection when there is one, otherwise the whole draft.
 *
 * Returns `null` when there is nothing to say, so the caller disables the control rather than
 * starting an operation that would fail.
 */
export function speechSourceText(
  draft: string,
  selection: { start: number; end: number } | null,
): string | null {
  const selected =
    selection && selection.end > selection.start
      ? draft.slice(selection.start, selection.end)
      : "";
  const candidate = selected.trim() ? selected : draft;
  const trimmed = candidate.trim();
  return trimmed ? trimmed : null;
}
