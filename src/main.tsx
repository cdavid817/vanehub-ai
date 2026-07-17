import React from "react";
import ReactDOM from "react-dom/client";
import "./i18n";
import "./styles.css";

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

void renderSurface();
