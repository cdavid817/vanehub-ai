import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { ErrorBoundary } from "react-error-boundary";
import { BrowserRouter, Navigate, Route, Routes, useLocation, useNavigate } from "react-router";
import { MainLayout } from "./main-layout/main-layout";
import { hasSeenLegacyRouteHint, markLegacyRouteHintSeen } from "./main-layout/legacy-route-hint";
import {
  extractReturnTo,
  legacyWorkbenchRedirectPath,
  parseWorkbenchLocation,
  recallWorkbenchPath,
  rememberWorkbenchLocation,
  withReturnTo,
  workbenchPath,
  type WorkbenchLocation,
} from "./main-layout/workbench-route";
import { SettingsShell } from "./settings/settings-shell";
import { SettingsProvider } from "./settings/settings-provider";
import { ThemeProvider } from "./theme/theme-provider";
import { useTranslation } from "react-i18next";
import { settingsService } from "./services/runtime-settings-client";
import { NotificationProvider, useNotifications } from "./notifications/notification-provider";
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
  const { t } = useTranslation();
  const { notify } = useNotifications();
  const workspaceLocation = useMemo(
    () => parseWorkbenchLocation(location.pathname, new URLSearchParams(location.search)),
    [location.pathname, location.search],
  );
  // 4.8: decoded fresh from the URL on every render, never stored — a stale returnTo surviving
  // past the navigation it was meant for would offer to send the reader somewhere unexpected.
  const returnTo = useMemo(() => extractReturnTo(new URLSearchParams(location.search)), [location.search]);

  // 4.5/4.14: a pre-redesign bookmark/history entry (e.g. `/workspace/loops`) parses as an
  // unrecognized destination and would otherwise silently land on Sessions — corrected here, in
  // the URL bar itself, rather than only in what renders at the old URL. The first time this ever
  // fires for this install, it also explains the jump with a one-time hint (never again after —
  // `hasSeenLegacyRouteHint` is a permanent dismissal flag, not a per-visit one).
  useEffect(() => {
    const redirect = legacyWorkbenchRedirectPath(location.pathname);
    if (!redirect) return;
    navigate(redirect, { replace: true });
    if (!hasSeenLegacyRouteHint()) {
      notify({ type: "info", title: t("app.legacyRouteHint.title"), message: t("app.legacyRouteHint.message"), scope: { kind: "global" } });
      markLegacyRouteHintSeen();
    }
  }, [location.pathname, navigate, notify, t]);

  useEffect(() => rememberWorkbenchLocation(workspaceLocation), [workspaceLocation]);

  // Takes a whole location so it depends only on `navigate`. An inline arrow closing over the
  // current location would change every render and re-fire the layout's reconciliation effect.
  const navigateWorkspace = useCallback(
    (next: WorkbenchLocation, options?: { replace?: boolean; returnTo?: WorkbenchLocation }) => {
      const path = options?.returnTo ? withReturnTo(workbenchPath(next), options.returnTo) : workbenchPath(next);
      navigate(path, options);
    },
    [navigate],
  );

  return (
    <MainLayout
      location={workspaceLocation}
      onConfigureOnePiece={() => navigate("/settings?section=agent-configurations&agentConfig=onepiece")}
      onNavigate={navigateWorkspace}
      onOpenSettings={(pageId) => navigate(pageId ? `/settings?section=${pageId}` : "/settings")}
      returnTo={returnTo}
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
