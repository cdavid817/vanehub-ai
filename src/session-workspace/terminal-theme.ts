function semanticColor(name: string, fallback: string) {
  const value = getComputedStyle(document.documentElement).getPropertyValue(name).trim();
  return value ? `hsl(${value})` : fallback;
}

export function createTerminalTheme() {
  const foreground = semanticColor("--foreground", "#111827");
  const primary = semanticColor("--primary", "#0284c7");

  return {
    background: "rgba(0, 0, 0, 0)",
    foreground,
    cursor: primary,
    selectionBackground: primary,
    selectionForeground: semanticColor("--primary-foreground", "#ffffff"),
    black: foreground,
    brightBlack: semanticColor("--muted-foreground", "#64748b"),
  };
}
