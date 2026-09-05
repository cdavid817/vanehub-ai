import type { WebglAddon } from "@xterm/addon-webgl";
import type { Terminal as XtermTerminal } from "@xterm/xterm";

/**
 * Moves a terminal off the DOM renderer, and reports how to undo it.
 *
 * xterm draws to DOM nodes unless a rendering addon takes over, which a CLI Agent repainting a full
 * TUI screen turns into thousands of node mutations per second.
 *
 * Loaded on demand rather than imported outright: the addon carries its own WebGL renderer, and
 * bundling that into the startup chunk would make every launch pay for it, including the launches
 * that never open a terminal. The terminal paints through the DOM renderer until the upgrade lands.
 *
 * WebGL2 is not always available -- software-rendered VMs, locked-down GPU drivers, and jsdom under
 * test all lack it -- and neither its absence nor a later context loss is worth failing a terminal
 * over, so both fall back to the renderer the terminal already had.
 *
 * Call after `terminal.open`: the addon needs the canvas that `open` creates.
 *
 * `onRendererChanged` runs once the swap has actually happened. Callers must refit and re-report
 * the terminal size from it: the renderers round cell metrics differently, so the rows and columns
 * computed before the swap — and already sent to the process on the other end — can be off by one,
 * which wraps or truncates a full-screen TUI for the rest of the session.
 */
export function attachAcceleratedRenderer(
  terminal: XtermTerminal,
  onRendererChanged: () => void,
): () => void {
  let addon: WebglAddon | null = null;
  let detached = false;
  void import("@xterm/addon-webgl")
    .then(({ WebglAddon: Addon }) => {
      if (detached) return;
      const next = new Addon();
      // Assigned before `loadAddon`, which activates the addon and can throw: xterm has already
      // taken ownership by then, so a handle dropped here would leak the renderer it registered.
      addon = next;
      next.onContextLoss(() => {
        next.dispose();
        addon = null;
        onRendererChanged();
      });
      terminal.loadAddon(next);
      onRendererChanged();
    })
    .catch(() => {
      addon?.dispose();
      addon = null;
    });
  return () => {
    detached = true;
    addon?.dispose();
    addon = null;
  };
}
