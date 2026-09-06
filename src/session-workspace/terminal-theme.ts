import type { ITheme } from "@xterm/xterm";

// No `ui-monospace` and no macOS-only names ahead of the Linux faces: on WebKitGTK an unknown
// family is handed to fontconfig, which returns its nearest guess instead of nothing, and
// `ui-monospace` came back as Noto Sans CJK — a wide proportional face whose cell width made
// every Latin character render letter-spaced. Chromium skips unknown names, which is why the
// Web build never showed it. Linux faces first (they do not exist on the other platforms, so
// they cost nothing there), then the macOS and Windows defaults, then the generic keyword.
export const terminalFontFamily =
  "'Noto Sans Mono', 'DejaVu Sans Mono', 'Liberation Mono', Menlo, Consolas, 'Cascadia Mono', monospace";

// Terminal palette tokens are full color values (hex), so read the raw computed
// value rather than wrapping it in hsl() like the app's semantic tokens.
function terminalColor(element: Element, name: string, fallback: string) {
  const value = getComputedStyle(element).getPropertyValue(name).trim();
  return value || fallback;
}

// The palette lives in CSS: a shared dark set on `:root`, and a light set that only resolves
// inside an Agent terminal container when `data-cli-terminal-theme="light"` is set on the root.
// Reading from the terminal's own element is therefore what keeps xterm and the CSS frame in
// agreement; callers that pass nothing (the ordinary Shell) keep the root defaults. The fallbacks
// are the dark values, for a test DOM with no stylesheet, and are not a second palette.
//
// Agent CLIs paint background-filled regions with 256-color/truecolor codes, so the embedded
// terminal renders on an opaque canvas with the complete 16-color ANSI palette instead of a
// transparent surface patched per ANSI class.
export function createTerminalTheme(element: Element = document.documentElement): ITheme {
  const color = (name: string, fallback: string) => terminalColor(element, name, fallback);
  return {
    background: color("--terminal-background", "#0d1117"),
    foreground: color("--terminal-foreground", "#c9d1d9"),
    cursor: color("--terminal-cursor", "#58a6ff"),
    cursorAccent: color("--terminal-background", "#0d1117"),
    selectionBackground: color("--terminal-selection", "#26456a"),
    selectionForeground: color("--terminal-selection-foreground", "#f0f6fc"),
    selectionInactiveBackground: color("--terminal-selection-inactive", "#1d3049"),
    scrollbarSliderBackground: color("--terminal-scrollbar-slider", "#484f5880"),
    scrollbarSliderHoverBackground: color("--terminal-scrollbar-slider-hover", "#6e7681a6"),
    scrollbarSliderActiveBackground: color("--terminal-scrollbar-slider-active", "#8b949ecc"),
    black: color("--terminal-ansi-black", "#484f58"),
    red: color("--terminal-ansi-red", "#ff7b72"),
    green: color("--terminal-ansi-green", "#3fb950"),
    yellow: color("--terminal-ansi-yellow", "#d29922"),
    blue: color("--terminal-ansi-blue", "#58a6ff"),
    magenta: color("--terminal-ansi-magenta", "#bc8cff"),
    cyan: color("--terminal-ansi-cyan", "#39c5cf"),
    white: color("--terminal-ansi-white", "#b1bac4"),
    brightBlack: color("--terminal-ansi-bright-black", "#6e7681"),
    brightRed: color("--terminal-ansi-bright-red", "#ffa198"),
    brightGreen: color("--terminal-ansi-bright-green", "#56d364"),
    brightYellow: color("--terminal-ansi-bright-yellow", "#e3b341"),
    brightBlue: color("--terminal-ansi-bright-blue", "#79c0ff"),
    brightMagenta: color("--terminal-ansi-bright-magenta", "#d2a8ff"),
    brightCyan: color("--terminal-ansi-bright-cyan", "#56d4dd"),
    brightWhite: color("--terminal-ansi-bright-white", "#f0f6fc"),
  };
}
