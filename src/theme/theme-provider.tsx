import { createContext, useContext, useMemo, type ReactNode } from "react";
import { useSettings } from "../settings/settings-provider";
import { ucdThemes, type UcdThemeId } from "./theme-registry";

interface ThemeContextValue {
  theme: UcdThemeId;
  setTheme: (theme: UcdThemeId) => void;
  themes: typeof ucdThemes;
}

const ThemeContext = createContext<ThemeContextValue | null>(null);

export function ThemeProvider({ children }: { children: ReactNode }) {
  const { settings, saveSetting } = useSettings();

  const value = useMemo(
    () => ({
      theme: settings.theme,
      setTheme: (theme: UcdThemeId) => {
        void saveSetting("theme", theme);
      },
      themes: ucdThemes,
    }),
    [saveSetting, settings.theme],
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
