/**
 * How often a streaming chat surface rebuilds its message array.
 *
 * Rebuilding re-renders the streaming row, and that row re-parses its whole accumulated Markdown --
 * remark, KaTeX and syntax highlighting -- every time. Rebuilding once per animation frame therefore
 * re-parses a long reply about sixty times a second.
 */
export const STREAM_RENDER_INTERVAL_MS = 100;

/**
 * Paces the rebuilds of one chat-event subscription.
 *
 * Shared by the two subscriptions rather than written twice: the main window and the floating
 * assistant render the same message components, so a pacing rule that lived in one of them would
 * leave the other re-parsing per frame, which is exactly how they had already drifted apart.
 *
 * A plain timer, deliberately. An animation frame is not delivered to a hidden or minimized window,
 * and this application closes to tray as its ordinary path — a frame armed just before that happens
 * would never fire, and every later event would queue behind it with nothing left to drain it.
 *
 * `performance.now()` because this measures an interval: the wall clock steps backwards on an NTP
 * correction or a resume from sleep, and a negative elapsed would stall the reply for the size of
 * the jump.
 */
export function createStreamRenderPacer(apply: () => void) {
  let timer = 0;
  let lastRunAt = 0;

  const run = () => {
    timer = 0;
    lastRunAt = performance.now();
    apply();
  };
  const cancel = () => {
    if (timer !== 0) {
      window.clearTimeout(timer);
      timer = 0;
    }
  };

  return {
    cancel,
    /** Applies immediately, dropping any pending pass. For terminal events, which cannot wait. */
    flushNow: () => {
      cancel();
      run();
    },
    schedule: () => {
      if (timer !== 0) return;
      const wait = Math.max(0, STREAM_RENDER_INTERVAL_MS - (performance.now() - lastRunAt));
      timer = window.setTimeout(run, wait);
    },
  };
}
