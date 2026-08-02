import React from "react";
import ReactDOM from "react-dom/client";
import { i18n } from "./i18n";
import "./styles.css";
import { recoverFromBootstrapFailure } from "./bootstrap-failure";

const floatingSurface = new URLSearchParams(window.location.search).get("surface") === "floating-assistant";
if (floatingSurface) {
  document.body.classList.add("floating-assistant-surface");
}

async function renderSurface() {
  const Surface = floatingSurface
    ? (await import("./floating-assistant/floating-assistant-root")).FloatingAssistantRoot
    : (await import("./App")).App;
  ReactDOM.createRoot(document.getElementById("root") as HTMLElement).render(
    <React.StrictMode>
      <Surface />
    </React.StrictMode>,
  );
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
