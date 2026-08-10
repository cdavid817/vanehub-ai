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

  if (runtimeKind === "web-http") {
    if (adapters.webHttp) {
      return withServiceErrorNormalization(adapters.webHttp as object) as T;
    }
    // An HTTP deployment was selected (the host set __VANEHUB_HTTP_BASE_URL__) but this
    // service has no HTTP adapter. Falling back to the mock here would silently return
    // fabricated data in production — the worst failure mode, since the app looks healthy
    // but every read is fake. Throw instead so the gap surfaces immediately at startup.
    throw new Error(
      `The "web-http" runtime was selected, but this service has no HTTP adapter. ` +
        `Either provide a webHttp adapter for this service, or unset __VANEHUB_HTTP_BASE_URL__ ` +
        `to fall back to the web mock.`,
    );
  }

  return withServiceErrorNormalization(adapters.webMock as object) as T;
}
