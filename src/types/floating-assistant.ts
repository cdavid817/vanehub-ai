export type FloatingAssistantSurfaceMode = "collapsed" | "menu" | "chat";
export type FloatingAssistantMainAction = "new-session" | "current-session" | "settings";

export interface FloatingAssistantAnchor {
  x: number;
  y: number;
  monitorName: string | null;
}

export interface FloatingAssistantConfig {
  enabled: boolean;
  anchor: FloatingAssistantAnchor | null;
}

export interface FloatingAssistantRuntimeInfo {
  nativeAvailable: boolean;
  platform: "windows" | "unsupported";
}

export type FloatingAssistantEvent =
  | { kind: "configuration-changed"; config: FloatingAssistantConfig }
  | { kind: "surface-changed"; mode: FloatingAssistantSurfaceMode }
  | { kind: "main-action"; action: FloatingAssistantMainAction }
  | { kind: "exit-requested" };
