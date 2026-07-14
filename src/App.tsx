import { MainLayout } from "./main-layout/main-layout";
import { ThemeProvider } from "./theme/theme-provider";

export function App() {
  return (
    <ThemeProvider>
      <MainLayout />
    </ThemeProvider>
  );
}
