import { existsSync, readFileSync, readdirSync, statSync } from "node:fs";
import path from "node:path";
import { describe, expect, it } from "vitest";

const projectRoot = path.resolve(import.meta.dirname, "..", "..");

/**
 * Tokens that exist only because the E2E fake exists. If any of them survives into a shipped
 * bundle, the fake shipped too, and whether it can be "activated" is already moot.
 */
const FAKE_TOKENS = [
  "__vanehubLocalMediaFake",
  "createDeterministicFakeLocalMediaService",
  "fixture recognized line one",
  "fixture transcript",
  "fixture-playback",
];

describe("the local-media E2E fake cannot reach a production build", () => {
  it("is proved absent from a real production bundle by its own gate", () => {
    // The two-build scan lives in `scripts/check-local-media-fake-bundle.mjs` and runs as its own
    // step. Keeping it out of the unit suite stops the canonical frontend gate from paying for two
    // Vite builds; this assertion pins that the gate still exists, still looks for every token,
    // and is still wired into package.json.
    const script = readFileSync(
      path.join(projectRoot, "scripts", "check-local-media-fake-bundle.mjs"),
      "utf8",
    );
    for (const token of FAKE_TOKENS) {
      expect(script, token).toContain(token);
    }
    const scripts = (
      JSON.parse(readFileSync(path.join(projectRoot, "package.json"), "utf8")) as {
        scripts: Record<string, string>;
      }
    ).scripts;
    expect(scripts["local-media:fake:check"]).toBe("node scripts/check-local-media-fake-bundle.mjs");
  });

  it("keeps the fake out of the composition root's production branch", () => {
    const source = readFileSync(
      path.join(projectRoot, "src", "services", "runtime-local-media-client.ts"),
      "utf8",
    );

    // A build-time constant is the whole mechanism. A runtime read of the environment, of storage,
    // of the URL, or of a global would each be a switch someone could flip in a shipped build.
    expect(source).toContain('import.meta.env.VITE_LOCAL_MEDIA_FAKE === "1"');
    expect(source).not.toMatch(/localStorage|sessionStorage|location\.search|URLSearchParams/);
    expect(source).not.toMatch(/window\.__|globalThis\.__/);
    expect(source).not.toMatch(/process\.env/);
  });

  it("is never reachable from production source outside the composition root", () => {
    const offenders: string[] = [];
    const walk = (dir: string) => {
      for (const entry of readdirSync(dir)) {
        const target = path.join(dir, entry);
        if (statSync(target).isDirectory()) {
          if (entry !== "testing") walk(target);
          continue;
        }
        if (!/\.(ts|tsx)$/.test(entry) || /\.(test|spec)\.(ts|tsx)$/.test(entry)) continue;
        if (target.endsWith(path.join("services", "runtime-local-media-client.ts"))) continue;
        if (readFileSync(target, "utf8").includes("local-media-e2e-fake")) {
          offenders.push(path.relative(projectRoot, target).replace(/\\/g, "/"));
        }
      }
    };
    walk(path.join(projectRoot, "src"));

    expect(offenders).toEqual([]);
  });

  it("keeps the browser suite's dev server free of the fake flag", () => {
    // The honest Web suite must keep asserting native-only behaviour, which it cannot do against a
    // server that serves the fake.
    const config = readFileSync(path.join(projectRoot, "playwright.config.ts"), "utf8");
    expect(config).not.toContain("VITE_LOCAL_MEDIA_FAKE");

    const scripts = (
      JSON.parse(readFileSync(path.join(projectRoot, "package.json"), "utf8")) as {
        scripts: Record<string, string>;
      }
    ).scripts;
    expect(scripts.dev).not.toContain("VITE_LOCAL_MEDIA_FAKE");
    expect(scripts.build).not.toContain("VITE_LOCAL_MEDIA_FAKE");
  });

  it("keeps the packaged desktop resources free of media fixtures", () => {
    const resources = path.join(projectRoot, "src-tauri", "resources");
    const offenders: string[] = [];
    const walk = (dir: string) => {
      if (!existsSync(dir)) return;
      for (const entry of readdirSync(dir)) {
        const target = path.join(dir, entry);
        if (statSync(target).isDirectory()) {
          if (entry !== "__pycache__") walk(target);
          continue;
        }
        // No fixture audio, no fixture transcript, no fixture model may ride along in the bundle.
        if (/\.(wav|mp3|onnx|pdmodel|pdiparams)$/i.test(entry)) {
          offenders.push(path.relative(projectRoot, target).replace(/\\/g, "/"));
        }
      }
    };
    walk(resources);

    expect(offenders).toEqual([]);
  });
});
