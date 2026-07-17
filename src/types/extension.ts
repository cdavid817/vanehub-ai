import type { OperationTask } from "./operation";

export const extensionCapabilityIds = ["ocr", "asr", "tts"] as const;
export type ExtensionCapabilityId = (typeof extensionCapabilityIds)[number];

export const extensionFrameworkIds = ["paddleocr", "faster-whisper", "sherpa-onnx"] as const;
export type ExtensionFrameworkId = (typeof extensionFrameworkIds)[number];

export type ExtensionLifecycleStatus =
  | "not-installed"
  | "installing"
  | "installed"
  | "starting"
  | "running"
  | "stopping"
  | "uninstalling"
  | "error"
  | "unsupported";

export interface ExtensionRequirement {
  runtime: string;
  packages: string[];
  estimatedDownloadMb: number;
  estimatedDiskMb: number;
  models: Array<{
    id: string;
    sizeMb: number;
    descriptionKey: string;
  }>;
}

export interface ExtensionFrameworkDefinition {
  id: ExtensionFrameworkId;
  capabilityId: ExtensionCapabilityId;
  nameKey: string;
  descriptionKey: string;
  defaultPort: number;
  requirement: ExtensionRequirement;
}

export interface ExtensionEnvironment {
  runtime: "tauri" | "web-mock";
  os: string;
  arch: string;
  supported: boolean;
  nativeOperationsAvailable: boolean;
  pythonPath: string | null;
  pythonVersion: string | null;
  reason: string | null;
}

export interface ExtensionFrameworkStatus {
  frameworkId: ExtensionFrameworkId;
  capabilityId: ExtensionCapabilityId;
  status: ExtensionLifecycleStatus;
  installed: boolean;
  enabled: boolean;
  running: boolean;
  port: number;
  installPath: string | null;
  installedVersion: string | null;
  lastHealthCheck: string | null;
  lastError: string | null;
  lastOperationId: string | null;
}

export interface ExtensionOverview {
  definitions: ExtensionFrameworkDefinition[];
  statuses: ExtensionFrameworkStatus[];
  environment: ExtensionEnvironment;
}

export interface ExtensionInstallPreview {
  frameworkId: ExtensionFrameworkId;
  supported: boolean;
  installPath: string;
  pythonPath: string | null;
  packages: string[];
  models: ExtensionRequirement["models"];
  estimatedDownloadMb: number;
  estimatedDiskMb: number;
  inferenceLocalOnly: boolean;
  reason: string | null;
}

export interface ExtensionFrameworkRequest {
  frameworkId: ExtensionFrameworkId;
}

export interface ExtensionEnableRequest extends ExtensionFrameworkRequest {
  enabled: boolean;
}

export interface ExtensionOperationResult {
  success: boolean;
  frameworkId: ExtensionFrameworkId;
  action: "install" | "uninstall" | "enable" | "disable" | "start" | "stop" | "self-test";
  message: string;
}

export function extensionOperationResult(task: OperationTask): ExtensionOperationResult | null {
  const value = task.result;
  if (!value || typeof value !== "object") return null;
  if (typeof value.success !== "boolean" || typeof value.frameworkId !== "string" || typeof value.action !== "string") {
    return null;
  }
  if (!extensionFrameworkIds.includes(value.frameworkId as ExtensionFrameworkId)) return null;
  const actions: ExtensionOperationResult["action"][] = [
    "install",
    "uninstall",
    "enable",
    "disable",
    "start",
    "stop",
    "self-test",
  ];
  if (!actions.includes(value.action as ExtensionOperationResult["action"])) return null;
  return {
    success: value.success,
    frameworkId: value.frameworkId as ExtensionFrameworkId,
    action: value.action as ExtensionOperationResult["action"],
    message: typeof value.message === "string" ? value.message : "",
  };
}
