import { detectRuntimeKind } from "./runtime-adapter";

/**
 * Whether this runtime's terminals are worth moving off xterm's DOM renderer.
 *
 * Only the desktop runtime is. The accelerated renderer exists for a CLI Agent repainting a full
 * TUI screen, which is thousands of DOM mutations a second; the Web/mock runtime writes fixture
 * text into its terminals and never streams a process at all, so it would pay to download a
 * renderer it cannot benefit from — and would put GPU-rasterized text inside the byte-exact
 * documentation screenshot comparison, where a runner image change would break a baseline in a way
 * that looks unrelated to whoever's branch hits it.
 *
 * A binding here rather than a check in the terminal surfaces: both surfaces render in both
 * runtimes, and `ARCH-FE-002` exists to keep that decision from being spelled out in each of them.
 */
export function acceleratesTerminalRendering(): boolean {
  return detectRuntimeKind() === "tauri";
}
