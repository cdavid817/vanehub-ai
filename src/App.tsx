import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { ErrorBoundary } from "react-error-boundary";
import { BrowserRouter, Navigate, Route, Routes, useNavigate } from "react-router-dom";
import { MainLayout } from "./main-layout/main-layout";
import { SettingsShell } from "./settings/settings-shell";
import { SettingsProvider } from "./settings/settings-provider";
import { ThemeProvider } from "./theme/theme-provider";
import { useTranslation } from "react-i18next";
import { settingsService } from "./services/runtime-settings-client";
import { NotificationProvider } from "./notifications/notification-provider";

const queryClient = new QueryClient({
  defaultOptions: {
    queries: {
      refetchOnWindowFocus: false,
      retry: 1,
    },
  },
});

function RouteErrorFallback({ error }: { error: unknown }) {
  const message = error instanceof Error ? error.message : String(error);
  const { t } = useTranslation();

  return (
    <main className="flex min-h-screen items-center justify-center bg-background p-6 text-foreground">
      <section className="ucd-panel max-w-lg rounded-lg p-5">
        <h1 className="text-base font-semibold">{t("app.error.title")}</h1>
        <p className="mt-2 text-sm text-muted-foreground">{message}</p>
      </section>
    </main>
  );
}

function WorkspaceRoute() {
  const navigate = useNavigate();

  return <MainLayout onOpenSettings={() => navigate("/settings")} />;
}

function SettingsRoute() {
  const navigate = useNavigate();

  return <SettingsShell onReturn={() => navigate("/workspace")} />;
}

export function App() {
  return (
    <SettingsProvider>
      <ThemeProvider>
        <NotificationProvider>
          <QueryClientProvider client={queryClient}>
            <BrowserRouter>
              <ErrorBoundary
              FallbackComponent={RouteErrorFallback}
              onError={(error, info) => {
                const message = error instanceof Error ? error.message : String(error);
                const stack = error instanceof Error ? error.stack : undefined;
                void settingsService.reportClientLogEvent({
                  level: "error",
                  kind: "error-boundary",
                  message,
                  source: "App",
                  stack,
                  details: { componentStack: info.componentStack ?? "" },
                });
              }}
              >
                <Routes>
                  <Route element={<WorkspaceRoute />} path="/workspace" />
                  <Route element={<SettingsRoute />} path="/settings" />
                  <Route element={<Navigate replace to="/workspace" />} path="*" />
                </Routes>
              </ErrorBoundary>
            </BrowserRouter>
          </QueryClientProvider>
        </NotificationProvider>
      </ThemeProvider>
    </SettingsProvider>
  );
}
