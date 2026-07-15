import { describe, expect, it } from "vitest";
import { readFileSync } from "node:fs";
import { fileURLToPath } from "node:url";
import { dirname, join } from "node:path";
import { createRuntimeAdapter, detectRuntimeKind } from "./runtime-adapter";

const currentDir = dirname(fileURLToPath(import.meta.url));

describe("runtime adapter selection", () => {
  it("selects the Tauri runtime when Tauri internals are present", () => {
    expect(detectRuntimeKind({ __TAURI_INTERNALS__: {} })).toBe("tauri");
  });

  it("selects the HTTP runtime when an HTTP base URL is configured", () => {
    expect(detectRuntimeKind({ __VANEHUB_HTTP_BASE_URL__: "http://127.0.0.1:8080" })).toBe("web-http");
  });

  it("falls back to the Web mock runtime", () => {
    expect(detectRuntimeKind(undefined)).toBe("web-mock");
  });

  it("returns the matching adapter and falls back to Web mock when HTTP is not implemented", () => {
    const adapters = {
      tauri: { name: "desktop" },
      webMock: { name: "mock" },
    };

    expect(createRuntimeAdapter(adapters, "tauri").name).toBe("desktop");
    expect(createRuntimeAdapter(adapters, "web-http").name).toBe("mock");
    expect(createRuntimeAdapter({ ...adapters, webHttp: { name: "http" } }, "web-http").name).toBe("http");
  });
});

describe("Web MCP adapter boundary", () => {
  it("does not import Tauri APIs", () => {
    const source = readFileSync(join(currentDir, "web-mcp-client.ts"), "utf8");

    expect(source).not.toContain("@tauri-apps/api");
    expect(source).not.toContain("invoke(");
  });
});
