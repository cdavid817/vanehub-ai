import { Buffer } from "node:buffer";
import { spawnSync } from "node:child_process";
import { mkdirSync, readFileSync, statSync, writeFileSync } from "node:fs";
import path from "node:path";
import process from "node:process";
import { fileURLToPath } from "node:url";
import { defaultScenario, writeScenario } from "./helpers/local-media-scenario.mjs";

const testsRoot = path.dirname(fileURLToPath(import.meta.url));

/** The test-only `paddleocr`, `faster_whisper` and `sherpa_onnx` packages. */
export const pythonFixtureRoot = path.join(testsRoot, "fixtures", "local-media-python");

/**
 * A 1x1 PNG.
 *
 * Real bytes rather than a renamed text file: admission sniffs the content and reads the declared
 * dimensions out of the IHDR before it will stage anything, so a placeholder would be refused by
 * the very checks this layer exists to exercise.
 */
const ONE_PIXEL_PNG =
  "iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAYAAAAfFcSJAAAADUlEQVR42mNk+M9QDwADhgGAWjR9awAAAABJRU5ErkJggg==";

const MANIFEST = "fixture.json";

/** Where everything this layer generates lives, derived from the run's own result directory. */
export function localMediaFixturePaths(resultDir) {
  const root = path.join(resultDir, "local-media-fixture");
  const models = path.join(root, "placeholder-models");
  return {
    root,
    manifest: path.join(root, MANIFEST),
    scenarioFile: path.join(root, "fixture-state", "scenario.json"),
    ocrSource: path.join(root, "source", "fixture-page.png"),
    ocrDetectionModelDir: path.join(models, "ocr-detection"),
    ocrRecognitionModelDir: path.join(models, "ocr-recognition"),
    sttModelDir: path.join(models, "whisper-model"),
    ttsModelPath: path.join(models, "voice.onnx"),
    ttsTokensPath: path.join(models, "tokens.txt"),
  };
}

/** Read the manifest the configuration wrote. Specs run in their own process and share no memory. */
export function readLocalMediaFixture(resultDir) {
  return JSON.parse(readFileSync(localMediaFixturePaths(resultDir).manifest, "utf8"));
}

/**
 * Absolute path to a working interpreter.
 *
 * The launcher requires an absolute file and refuses a bare name, so `python` on `PATH` is not an
 * answer -- and asking the interpreter for its own `sys.executable` is the only way to learn where
 * a launcher such as `py` actually landed.
 */
export function resolvePythonExecutable() {
  const candidates = process.env.VANEHUB_PYTHON
    ? [process.env.VANEHUB_PYTHON]
    : process.platform === "win32"
      ? ["python", "py", "python3"]
      : ["python3", "python"];
  const attempted = [];
  for (const candidate of candidates) {
    const probed = spawnSync(candidate, ["-c", "import sys; print(sys.executable)"], {
      encoding: "utf8",
    });
    const resolved = probed.status === 0 ? (probed.stdout ?? "").trim() : "";
    if (resolved && path.isAbsolute(resolved) && isFile(resolved)) return resolved;
    attempted.push(`${candidate}: ${probed.status === 0 ? resolved || "no path" : "not runnable"}`);
  }
  // Refusing beats degrading. This layer's whole claim is that the real worker ran, and a suite
  // that quietly passed without an interpreter would be evidence of nothing.
  throw new Error(
    `BLOCKED: the local-media fixture layer needs a Python interpreter (tried ${attempted.join("; ")}). ` +
      "Set VANEHUB_PYTHON to an absolute interpreter path.",
  );
}

function isFile(candidate) {
  try {
    return statSync(candidate).isFile();
  } catch {
    return false;
  }
}

/**
 * Build this run's fixture tree and record it for the specs.
 *
 * The model paths are empty placeholders that genuinely exist. The worker resolves and stats every
 * configured model path before it constructs an engine, so real directories and real files are what
 * keep that production check in the picture; their contents are never read, because the third-party
 * libraries that would read them are the ones being stood in for.
 */
export function prepareLocalMediaFixture(resultDir) {
  const paths = localMediaFixturePaths(resultDir);
  const directories = [
    paths.root,
    path.dirname(paths.scenarioFile),
    path.dirname(paths.ocrSource),
    paths.ocrDetectionModelDir,
    paths.ocrRecognitionModelDir,
    paths.sttModelDir,
  ];
  for (const directory of directories) mkdirSync(directory, { recursive: true });

  const placeholder = "vanehub placeholder — the fixture library never reads this file\n";
  writeFileSync(paths.ttsModelPath, placeholder);
  writeFileSync(paths.ttsTokensPath, placeholder);
  writeFileSync(paths.ocrSource, Buffer.from(ONE_PIXEL_PNG, "base64"));
  writeScenario(paths.scenarioFile, defaultScenario());

  const manifest = { ...paths, pythonExecutable: resolvePythonExecutable(), pythonFixtureRoot };
  writeFileSync(paths.manifest, `${JSON.stringify(manifest, null, 2)}\n`);
  return manifest;
}

/**
 * The worker-visible switches.
 *
 * Every one of these is read from a module that only compiles under `desktop-e2e`, and the runtime
 * flag has to be `1` on top of that. Nothing in a production build looks at any of these names.
 */
export function localMediaFixtureEnvironment(manifest) {
  return {
    VANEHUB_LOCAL_MEDIA_E2E_FIXTURES: "1",
    VANEHUB_LOCAL_MEDIA_E2E_SCENARIO_FILE: manifest.scenarioFile,
    VANEHUB_LOCAL_MEDIA_E2E_PYTHON_ROOT: manifest.pythonFixtureRoot,
    VANEHUB_LOCAL_MEDIA_E2E_OCR_SOURCE: manifest.ocrSource,
  };
}
