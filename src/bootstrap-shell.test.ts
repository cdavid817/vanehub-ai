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
    expect(html).toContain('role="status"');
    expect(html).toContain("Starting...");
    expect(shellPosition).toBeGreaterThan(-1);
    expect(entryPosition).toBeGreaterThan(shellPosition);
  });

  it("keeps the startup styles independent from the application bundle", () => {
    const css = readFileSync(`${projectRoot}/public/bootstrap.css`, "utf8");

    expect(css).toContain(".bootstrap-shell");
    expect(css).toContain("position: fixed");
    expect(css).toContain("background: #07111f");
    expect(css).toContain("prefers-reduced-motion: reduce");
  });

  it("keeps the static shell visible until React mounts", () => {
    const entry = readFileSync(`${projectRoot}/src/main.tsx`, "utf8");

    expect(entry).not.toContain("renderStartupLoading");
    expect(entry).not.toContain('i18n.t("featureLoad.loading")');
    expect(entry).toContain('document.getElementById("bootstrap-shell")?.remove()');
    expect(entry.indexOf('root.dataset.vanehubBootstrap = "ready"')).toBeGreaterThan(
      entry.indexOf('document.getElementById("bootstrap-shell")?.remove()'),
    );
  });

  it("matches the native window background to the startup overlay", () => {
    const config = readFileSync(`${projectRoot}/src-tauri/tauri.conf.json`, "utf8");

    expect(config).toContain('"backgroundColor": "#07111F"');
  });
});
