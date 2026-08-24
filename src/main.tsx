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

function renderStartupLoading(root: HTMLElement) {
  const loading = document.createElement("div");
  loading.className = "grid min-h-screen place-items-center bg-background px-6 text-foreground";
  loading.setAttribute("aria-live", "polite");
  loading.setAttribute("role", "status");
  const content = document.createElement("div");
  content.className = "flex items-center gap-3 rounded-lg border border-border bg-card px-5 py-4 shadow-sm";
  const spinner = document.createElement("span");
  spinner.className = "h-5 w-5 animate-spin rounded-full border-2 border-primary border-t-transparent";
  spinner.setAttribute("aria-hidden", "true");
  const label = document.createElement("span");
  label.className = "text-sm font-medium";
  label.textContent = i18n.t("featureLoad.loading");
  content.append(spinner, label);
  loading.append(content);
  root.replaceChildren(loading);
}

async function renderSurface() {
  const root = document.getElementById("root") as HTMLElement;
  renderStartupLoading(root);
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
      <Surface />
    </React.StrictMode>,
  );
  root.dataset.vanehubBootstrap = "ready";
}

void renderSurface().catch((error: unknown) => {
  recoverFromBootstrapFailure({
    root: document.getElementById("root") as HTMLElement,
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
});
