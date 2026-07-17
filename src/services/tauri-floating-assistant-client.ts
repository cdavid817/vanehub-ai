import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { getCurrentWindow } from "@tauri-apps/api/window";
import type { FloatingAssistantConfig, FloatingAssistantEvent, FloatingAssistantRuntimeInfo } from "../types/floating-assistant";
import type { FloatingAssistantService } from "./floating-assistant-service";

export const tauriFloatingAssistantClient: FloatingAssistantService = {
  getRuntimeInfo() {
    return invoke<FloatingAssistantRuntimeInfo>("get_floating_assistant_runtime_info");
  },
  getConfig() {
    return invoke<FloatingAssistantConfig>("get_floating_assistant_config");
  },
  setEnabled(enabled) {
    return invoke<FloatingAssistantConfig>("set_floating_assistant_enabled", { enabled });
  },
  async setSurfaceMode(mode) {
    await invoke("set_floating_assistant_surface", { mode });
  },
  async startDragging() {
    await invoke("start_floating_assistant_drag");
  },
  saveAnchor(anchor) {
    return invoke<FloatingAssistantConfig>("save_floating_assistant_anchor", { anchor });
  },
  async showMainWindow(action) {
    await invoke("show_main_window", { action });
  },
  async exitApplication() {
    await invoke("exit_application");
  },
  async subscribeEvents(handler) {
    const cleanups: Array<() => void> = [];
    cleanups.push(await listen<FloatingAssistantEvent>("floating-assistant:event", (event) => handler(event.payload)));
    const currentWindow = getCurrentWindow();
    if (currentWindow.label === "floating-assistant") {
      let persistTimer: ReturnType<typeof setTimeout> | undefined;
      cleanups.push(await currentWindow.onMoved(() => {
        if (persistTimer) clearTimeout(persistTimer);
        persistTimer = setTimeout(() => {
          void invoke("persist_floating_assistant_position");
        }, 250);
      }));
      cleanups.push(() => {
        if (persistTimer) clearTimeout(persistTimer);
      });
    }
    return () => cleanups.forEach((cleanup) => cleanup());
  },
};
