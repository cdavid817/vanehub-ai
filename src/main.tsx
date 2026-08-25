import React from "react";
import ReactDOM from "react-dom/client";
import { i18n } from "./i18n";
import "./styles.css";
import { recoverFromBootstrapFailure } from "./bootstrap-failure";

const FATAL_FRONTEND_DIAGNOSTIC_LIMIT = 240;

function redactFatalFrontendDiagnostic(value: unknown) {
  const message = value instanceof Error ? value.message : typeof value === "string" ? value : "unavailable";
  return message
    .replace(/((?:api[-_]?key|token|secret|password|authorization)\s*[:=]\s*)[^\s,;]+/giu, "$1[REDACTED]")
    .replace(/(bearer\s+)[^\s,;]+/giu, "$1[REDACTED]")
    .slice(0, FATAL_FRONTEND_DIAGNOSTIC_LIMIT);
}

const floatingSurface = new URLSearchParams(window.location.search).get("surface") === "floating-assistant";
if (floatingSurface) {
  document.body.classList.add("floating-assistant-surface");
}

function SurfaceReady({ children, root }: { children: React.ReactNode; root: HTMLElement }) {
  React.useEffect(() => {
    document.getElementById("bootstrap-shell")?.remove();
    root.dataset.vanehubBootstrap = "ready";
  }, [root]);
  return children;
}

async function renderSurface() {
  const root = document.getElementById("root") as HTMLElement;
  if (import.meta.env.VITE_DESKTOP_E2E === "1") {
    const markFatalFrontendError = (kind: string, detail: unknown) => {
      root.dataset.vanehubFatalError = kind;
      root.dataset.vanehubFatalErrorDetail = redactFatalFrontendDiagnostic(detail);
    };
    window.addEventListener("error", (event) => markFatalFrontendError("error", event.error ?? event.message));
    window.addEventListener("unhandledrejection", (event) => markFatalFrontendError("unhandledrejection", event.reason));
    await import("@wdio/tauri-plugin");
  }
  const Surface = floatingSurface
    ? (await import("./floating-assistant/floating-assistant-root")).FloatingAssistantRoot
    : (await import("./App")).App;
  ReactDOM.createRoot(root).render(
    <React.StrictMode>
      <SurfaceReady root={root}>
        <Surface />
      </SurfaceReady>
    </React.StrictMode>,
  );
}

void renderSurface().catch((error: unknown) => {
  const root = document.getElementById("root") as HTMLElement;
  recoverFromBootstrapFailure({
    root,
    copy: {
      title: i18n.t("app.bootstrapFailure.title"),
      description: i18n.t("app.bootstrapFailure.description"),
      retry: i18n.t("app.bootstrapFailure.retry"),
    },
    error,
    surface: floatingSurface ? "floating-assistant" : "main",
    retry: () => window.location.reload(),
    report: async (event) => {
      const { settingsService } = await import("./services/runtime-settings-client");
      await settingsService.reportClientLogEvent(event);
    },
  });
  window.requestAnimationFrame(() => {
    document.getElementById("bootstrap-shell")?.remove();
    root.dataset.vanehubBootstrap = "failed";
  });
});
