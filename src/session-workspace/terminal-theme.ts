// Terminal palette tokens are full color values (hex), so read the raw computed
// value rather than wrapping it in hsl() like the app's semantic tokens.
function terminalColor(name: string, fallback: string) {
  const value = getComputedStyle(document.documentElement).getPropertyValue(name).trim();
  return value || fallback;
}

// Agent CLIs are dark-themed full-screen TUIs that paint background-filled
// regions with 256-color/truecolor codes, so the embedded terminal renders on
// an opaque dark canvas with the complete 16-color ANSI palette instead of a
// transparent surface patched per ANSI class.
export function createTerminalTheme() {
  return {
    background: terminalColor("--terminal-background", "#0d1117"),
    foreground: terminalColor("--terminal-foreground", "#c9d1d9"),
    cursor: terminalColor("--terminal-cursor", "#58a6ff"),
    cursorAccent: terminalColor("--terminal-background", "#0d1117"),
    selectionBackground: terminalColor("--terminal-selection", "#26456a"),
    selectionForeground: terminalColor("--terminal-selection-foreground", "#f0f6fc"),
    black: terminalColor("--terminal-ansi-black", "#484f58"),
    red: terminalColor("--terminal-ansi-red", "#ff7b72"),
    green: terminalColor("--terminal-ansi-green", "#3fb950"),
    yellow: terminalColor("--terminal-ansi-yellow", "#d29922"),
    blue: terminalColor("--terminal-ansi-blue", "#58a6ff"),
    magenta: terminalColor("--terminal-ansi-magenta", "#bc8cff"),
    cyan: terminalColor("--terminal-ansi-cyan", "#39c5cf"),
    white: terminalColor("--terminal-ansi-white", "#b1bac4"),
    brightBlack: terminalColor("--terminal-ansi-bright-black", "#6e7681"),
    brightRed: terminalColor("--terminal-ansi-bright-red", "#ffa198"),
    brightGreen: terminalColor("--terminal-ansi-bright-green", "#56d364"),
    brightYellow: terminalColor("--terminal-ansi-bright-yellow", "#e3b341"),
    brightBlue: terminalColor("--terminal-ansi-bright-blue", "#79c0ff"),
    brightMagenta: terminalColor("--terminal-ansi-bright-magenta", "#d2a8ff"),
    brightCyan: terminalColor("--terminal-ansi-bright-cyan", "#56d4dd"),
    brightWhite: terminalColor("--terminal-ansi-bright-white", "#f0f6fc"),
  };
}
