import { readFileSync } from "node:fs";
import { fileURLToPath } from "node:url";
import { describe, expect, it } from "vitest";

const projectRoot = fileURLToPath(new URL("../", import.meta.url));

describe("static bootstrap shell", () => {
  it("renders feedback before the React entry module is ready", () => {
    const html = readFileSync(`${projectRoot}/index.html`, "utf8");
    const shellPosition = html.indexOf('class="bootstrap-shell"');
    const entryPosition = html.indexOf('src="/src/main.tsx"');

    expect(html).toContain('href="/bootstrap.css"');
    expect(html).toContain('id="bootstrap-shell"');
    expect(html).toContain('id="root" data-vanehub-bootstrap="starting"');
    expect(html).toContain('role="status"');
    expect(html).toContain("Starting...");
    expect(shellPosition).toBeGreaterThan(-1);
    expect(entryPosition).toBeGreaterThan(shellPosition);
  });

  it("keeps the startup styles independent from the application bundle", () => {
    const css = readFileSync(`${projectRoot}/public/bootstrap.css`, "utf8");

    expect(css).toContain(".bootstrap-shell");
    expect(css).toContain("position: fixed");
    expect(css).toContain("#f4f7fb");
    expect(css).not.toContain("#07111f");
    expect(css).toContain("prefers-reduced-motion: reduce");
  });

  it("keeps the static shell visible until React renders application content", () => {
    const entry = readFileSync(`${projectRoot}/src/main.tsx`, "utf8");

    expect(entry).toContain("watchSurfaceReadiness(root");
    expect(entry).toContain('document.getElementById("bootstrap-shell")?.remove()');
    expect(entry.indexOf('root.dataset.vanehubBootstrap = "ready"')).toBeGreaterThan(
      entry.indexOf('document.getElementById("bootstrap-shell")?.remove()'),
    );
  });

  it("matches the native window background to the startup overlay", () => {
    const config = readFileSync(`${projectRoot}/src-tauri/tauri.conf.json`, "utf8");
    const runtime = readFileSync(`${projectRoot}/src-tauri/src/bootstrap/runtime.rs`, "utf8");

    expect(config).toContain('"visible": false');
    expect(config).toContain('"backgroundColor": "#F4F7FB"');
    expect(runtime).toContain(".on_page_load(show_main_window_after_page_load)");
    expect(runtime).toContain('webview.label() != "main"');
    expect(runtime).toContain("payload.event() != PageLoadEvent::Finished");
  });
});
