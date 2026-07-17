import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { SettingsProvider } from "../settings/settings-provider";
import { ThemeProvider } from "../theme/theme-provider";
import { FloatingAssistantApp } from "./floating-assistant-app";

const floatingQueryClient = new QueryClient({
  defaultOptions: {
    queries: { refetchOnWindowFocus: false, retry: 1 },
  },
});

export function FloatingAssistantRoot() {
  return (
    <SettingsProvider>
      <ThemeProvider>
        <QueryClientProvider client={floatingQueryClient}>
          <FloatingAssistantApp />
        </QueryClientProvider>
      </ThemeProvider>
    </SettingsProvider>
  );
}
