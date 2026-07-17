import type {
  FloatingAssistantAnchor,
  FloatingAssistantConfig,
  FloatingAssistantEvent,
  FloatingAssistantMainAction,
  FloatingAssistantRuntimeInfo,
  FloatingAssistantSurfaceMode,
} from "../types/floating-assistant";

export interface FloatingAssistantService {
  getRuntimeInfo(): Promise<FloatingAssistantRuntimeInfo>;
  getConfig(): Promise<FloatingAssistantConfig>;
  setEnabled(enabled: boolean): Promise<FloatingAssistantConfig>;
  setSurfaceMode(mode: FloatingAssistantSurfaceMode): Promise<void>;
  startDragging(): Promise<void>;
  saveAnchor(anchor: FloatingAssistantAnchor): Promise<FloatingAssistantConfig>;
  showMainWindow(action: FloatingAssistantMainAction): Promise<void>;
  exitApplication(): Promise<void>;
  subscribeEvents(handler: (event: FloatingAssistantEvent) => void): Promise<() => void>;
}
