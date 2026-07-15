import { withServiceErrorNormalization } from "./service-error";

export type RuntimeKind = "tauri" | "web-mock" | "web-http";

export interface RuntimeAdapterSet<T extends object> {
  tauri: T;
  webMock: T;
  webHttp?: T;
}

declare global {
  interface Window {
    __TAURI_INTERNALS__?: unknown;
    __VANEHUB_RUNTIME__?: RuntimeKind;
    __VANEHUB_HTTP_BASE_URL__?: string;
  }
}

export function detectRuntimeKind(host: Pick<Window, "__TAURI_INTERNALS__" | "__VANEHUB_RUNTIME__" | "__VANEHUB_HTTP_BASE_URL__"> | undefined = typeof window === "undefined" ? undefined : window): RuntimeKind {
  if (host?.__VANEHUB_RUNTIME__) {
    return host.__VANEHUB_RUNTIME__;
  }

  if (host?.__TAURI_INTERNALS__) {
    return "tauri";
  }

  if (host?.__VANEHUB_HTTP_BASE_URL__) {
    return "web-http";
  }

  return "web-mock";
}

export function createRuntimeAdapter<T extends object>(adapters: RuntimeAdapterSet<T>, runtimeKind = detectRuntimeKind()): T {
  if (runtimeKind === "tauri") {
    return withServiceErrorNormalization(adapters.tauri as object) as T;
  }

  if (runtimeKind === "web-http" && adapters.webHttp) {
    return withServiceErrorNormalization(adapters.webHttp as object) as T;
  }

  return withServiceErrorNormalization(adapters.webMock as object) as T;
}
