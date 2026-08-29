import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { ErrorBoundary } from "react-error-boundary";
import { BrowserRouter, Navigate, Route, Routes, useLocation, useNavigate } from "react-router";
import { MainLayout } from "./main-layout/main-layout";
import {
  parseWorkspaceLocation,
  recallWorkspacePath,
  rememberWorkspaceLocation,
  workspacePath,
  type WorkspaceLocation,
} from "./main-layout/workspace-route";
import { SettingsShell } from "./settings/settings-shell";
import { SettingsProvider } from "./settings/settings-provider";
import { ThemeProvider } from "./theme/theme-provider";
import { useTranslation } from "react-i18next";
import { settingsService } from "./services/runtime-settings-client";
import { NotificationProvider } from "./notifications/notification-provider";
import { CuratorNotificationBridge } from "./notifications/curator-notification-bridge";
import { GenerationNotificationBridge } from "./notifications/generation-notification-bridge";
import { EvolutionNotificationBridge } from "./notifications/evolution-notification-bridge";
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
  const workspaceLocation = useMemo(() => parseWorkspaceLocation(location.pathname), [location.pathname]);

  useEffect(() => rememberWorkspaceLocation(workspaceLocation), [workspaceLocation]);

  // Takes a whole location so it depends only on `navigate`. An inline arrow closing over the
  // current location would change every render and re-fire the layout's reconciliation effect.
  const navigateWorkspace = useCallback(
    (next: WorkspaceLocation, options?: { replace?: boolean }) => navigate(workspacePath(next), options),
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
      if (event.action === "new-session") navigate(workspacePath({ destination: "sessions", creatingSession: true }));
      else if (event.action === "current-session") navigate(recallWorkspacePath());
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
  return <Navigate replace to={recallWorkspacePath()} />;
}

function SettingsRoute() {
  const navigate = useNavigate();
  const location = useLocation();
  const params = new URLSearchParams(location.search);
  const onePieceRequested = params.get("agentConfig") === "onepiece";
  const curatorRequested = params.get("skillWorkspace") === "curator";
  const generationRequested = params.get("skillWorkspace") === "generation";
  const evolutionRequested = params.get("skillWorkspace") === "orchestration";
  const overlayRequested = Boolean(params.get("overlayHistory") && params.get("skill"));
  const navigationTarget = evolutionRequested ? {
    evolutionWorkspaceId: params.get("workspace") ?? undefined,
    evolutionRunId: params.get("evolutionRun") ?? undefined,
    evolutionApplicationId: params.get("evolutionApplication") ?? undefined,
    evolutionProbationId: params.get("evolutionProbation") ?? undefined,
    evolutionBreakerId: params.get("evolutionBreaker") ?? undefined,
  } : generationRequested ? {
    generationWorkspaceId: params.get("workspace") ?? undefined,
    generationJobId: params.get("generationJob") ?? undefined,
  } : curatorRequested ? {
    curatorCandidateId: params.get("candidate") ?? undefined,
    curatorWorkspaceId: params.get("workspace") ?? undefined,
    overlayHistoryId: params.get("overlayHistory") ?? undefined,
  } : overlayRequested ? {
    overlayHistoryId: params.get("overlayHistory") ?? undefined,
    overlaySkillId: params.get("skill") ?? undefined,
  } : onePieceRequested ? { agentConfigAgentId: "onepiece" as const } : null;

  return (
    <SettingsShell
      initialNavigationTarget={navigationTarget}
      initialPageId={onePieceRequested ? "agent-configurations" : curatorRequested || generationRequested || evolutionRequested || overlayRequested ? "skills" : undefined}
      onReturn={() => navigate("/workspace")}
    />
  );
}

export function App() {
  return (
    <SettingsProvider>
      <ThemeProvider>
        <QueryClientProvider client={queryClient}>
          <BrowserRouter>
            <RoutedProviders />
          </BrowserRouter>
        </QueryClientProvider>
      </ThemeProvider>
    </SettingsProvider>
  );
}

function RoutedProviders() {
  const navigate = useNavigate();
  return (
    <NotificationProvider onNavigate={navigate}>
      <CuratorNotificationBridge />
      <GenerationNotificationBridge />
      <EvolutionNotificationBridge />
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
    </NotificationProvider>
  );
}
