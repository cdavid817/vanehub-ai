import assert from "node:assert/strict";
import { mkdtemp } from "node:fs/promises";
import test from "node:test";
import { tmpdir } from "node:os";
import path from "node:path";
import process from "node:process";
import { createDesktopConfig } from "./wdio-shared.mjs";

test("delays worker startup so the Tauri service can replace a draining embedded driver", async () => {
  const resultDir = await mkdtemp(path.join(tmpdir(), "vanehub-wdio-shared-"));
  const previousArtifact = process.env.VANEHUB_DESKTOP_ARTIFACT;
  const previousResultDir = process.env.VANEHUB_DESKTOP_RESULT_DIR;
  process.env.VANEHUB_DESKTOP_ARTIFACT = "/tmp/vanehub-desktop-test-artifact";
  process.env.VANEHUB_DESKTOP_RESULT_DIR = resultDir;

  try {
    const config = await createDesktopConfig({ specDirectory: "specs", specFiles: ["smoke.e2e.mjs"] });
    const startedAt = Date.now();
    await config.onWorkerStart();
    assert.ok(Date.now() - startedAt >= 1_900);
  } finally {
    if (previousArtifact === undefined) delete process.env.VANEHUB_DESKTOP_ARTIFACT;
    else process.env.VANEHUB_DESKTOP_ARTIFACT = previousArtifact;
    if (previousResultDir === undefined) delete process.env.VANEHUB_DESKTOP_RESULT_DIR;
    else process.env.VANEHUB_DESKTOP_RESULT_DIR = previousResultDir;
  }
});
