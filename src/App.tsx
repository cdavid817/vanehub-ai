import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { ErrorBoundary } from "react-error-boundary";
import { BrowserRouter, Navigate, Route, Routes, useLocation, useNavigate } from "react-router";
import { MainLayout } from "./main-layout/main-layout";
import {
  parseWorkbenchLocation,
  recallWorkbenchPath,
  rememberWorkbenchLocation,
  workbenchPath,
  type WorkbenchLocation,
} from "./main-layout/workbench-route";
import { SettingsShell } from "./settings/settings-shell";
import { SettingsProvider } from "./settings/settings-provider";
import { ThemeProvider } from "./theme/theme-provider";
import { useTranslation } from "react-i18next";
import { settingsService } from "./services/runtime-settings-client";
import { NotificationProvider } from "./notifications/notification-provider";
import { floatingAssistantService } from "./services/runtime-floating-assistant-client";
import { useCallback, useEffect, useMemo } from "react";

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

/**
 * One route for every workspace URL. Destinations are read from the path inside `MainLayout`
 * rather than being separate route elements, because React Router unmounts the previous element
 * on navigation and the workspace depends on visited destinations staying mounted.
 */
function WorkspaceRoute() {
  const navigate = useNavigate();
  const location = useLocation();
  const workspaceLocation = useMemo(
    () => parseWorkbenchLocation(location.pathname, new URLSearchParams(location.search)),
    [location.pathname, location.search],
  );

  useEffect(() => rememberWorkbenchLocation(workspaceLocation), [workspaceLocation]);

  // Takes a whole location so it depends only on `navigate`. An inline arrow closing over the
  // current location would change every render and re-fire the layout's reconciliation effect.
  const navigateWorkspace = useCallback(
    (next: WorkbenchLocation, options?: { replace?: boolean }) => navigate(workbenchPath(next), options),
    [navigate],
  );

  return (
    <MainLayout
      location={workspaceLocation}
      onConfigureOnePiece={() => navigate("/settings?section=agent-configurations&agentConfig=onepiece")}
      onNavigate={navigateWorkspace}
      onOpenSettings={(pageId) => navigate(pageId ? `/settings?section=${pageId}` : "/settings")}
    />
  );
}

function AppRoutes() {
  const navigate = useNavigate();

  useEffect(() => {
    let active = true;
    let cleanup: (() => void) | undefined;
    void floatingAssistantService.subscribeEvents((event) => {
      if (event.kind !== "main-action") return;
      if (event.action === "new-session") navigate(workbenchPath({ destination: "sessions", sessionId: null, creatingSession: true }));
      else if (event.action === "current-session") navigate(recallWorkbenchPath());
      else navigate("/settings");
    }).then((unsubscribe) => {
      if (active) cleanup = unsubscribe;
      else unsubscribe();
    });
    return () => {
      active = false;
      cleanup?.();
    };
  }, [navigate]);

  return (
    <Routes>
      <Route element={<WorkspaceRoute />} path="/workspace/*" />
      <Route element={<SettingsRoute />} path="/settings" />
      <Route element={<LaunchRedirect />} path="*" />
    </Routes>
  );
}

/** Resumes where the previous session stopped instead of always landing on an empty workspace. */
function LaunchRedirect() {
  return <Navigate replace to={recallWorkbenchPath()} />;
}

function SettingsRoute() {
  const navigate = useNavigate();
  const location = useLocation();
  const onePieceRequested = new URLSearchParams(location.search).get("agentConfig") === "onepiece";

  return (
    <SettingsShell
      initialNavigationTarget={onePieceRequested ? { agentConfigAgentId: "onepiece" } : null}
      initialPageId={onePieceRequested ? "agent-configurations" : undefined}
      onOpenSession={(sessionId) => navigate(`/workspace/sessions/${encodeURIComponent(sessionId)}`)}
      onReturn={() => navigate("/workspace")}
    />
  );
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
                <AppRoutes />
              </ErrorBoundary>
            </BrowserRouter>
          </QueryClientProvider>
        </NotificationProvider>
      </ThemeProvider>
    </SettingsProvider>
  );
}
