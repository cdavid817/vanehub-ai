import { createContext, useContext, useEffect, useMemo, useState, type ReactNode } from "react";
import { defaultThemeId, normalizeThemeId, themeStorageKey, ucdThemes, type UcdThemeId } from "./theme-registry";

interface ThemeContextValue {
  theme: UcdThemeId;
  setTheme: (theme: UcdThemeId) => void;
  themes: typeof ucdThemes;
}

const ThemeContext = createContext<ThemeContextValue | null>(null);

function readStoredTheme(): UcdThemeId {
  if (typeof window === "undefined") return defaultThemeId;
  return normalizeThemeId(window.localStorage.getItem(themeStorageKey));
}

export function ThemeProvider({ children }: { children: ReactNode }) {
  const [theme, setThemeState] = useState<UcdThemeId>(readStoredTheme);

  useEffect(() => {
    document.documentElement.dataset.theme = theme;
    window.localStorage.setItem(themeStorageKey, theme);
  }, [theme]);

  const value = useMemo(
    () => ({
      theme,
      setTheme: setThemeState,
      themes: ucdThemes,
    }),
    [theme],
  );

  return <ThemeContext.Provider value={value}>{children}</ThemeContext.Provider>;
}

export function useTheme() {
  const value = useContext(ThemeContext);
  if (!value) {
    throw new Error("useTheme must be used inside ThemeProvider");
  }
  return value;
}
