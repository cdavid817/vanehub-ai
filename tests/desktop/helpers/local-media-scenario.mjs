import { closeSync, fsyncSync, mkdirSync, openSync, renameSync, rmSync, writeSync } from "node:fs";
import path from "node:path";
import process from "node:process";

/** The engine section names the Python fixtures look themselves up under. */
export const ENGINE_SECTIONS = ["paddleocr", "faster-whisper", "sherpa-onnx"];

/** The device and hardware sections the Rust fixtures read. */
const HOST_SECTIONS = ["capture", "playback", "devices"];

let sequence = 0;

/**
 * Write the scenario document both the Rust fixtures and the Python packages read.
 *
 * Atomic, because both sides re-read the file on every interaction so a scenario can change while a
 * worker is alive -- which makes a half-written document genuinely reachable. Both readers treat
 * malformed JSON as a hard configuration error, so a torn write would abort the application instead
 * of producing a legible assertion failure.
 */
export function writeScenario(scenarioFile, scenario) {
  const directory = path.dirname(scenarioFile);
  mkdirSync(directory, { recursive: true });
  sequence += 1;
  const temporary = path.join(directory, `scenario-next-${process.pid}-${sequence}.json`);
  const handle = openSync(temporary, "w");
  try {
    writeSync(handle, `${JSON.stringify(scenario, null, 2)}\n`);
    fsyncSync(handle);
  } finally {
    closeSync(handle);
  }
  renameSync(temporary, scenarioFile);
}

export function defaultScenario() {
  const scenario = {};
  for (const section of [...HOST_SECTIONS, ...ENGINE_SECTIONS]) {
    scenario[section] = { behaviour: "success" };
  }
  return scenario;
}

/** Where the Python fixtures leave their process-level evidence. */
export function markerPath(scenarioFile, engine, name) {
  return `${scenarioFile}.${engine}.${name}`;
}

/**
 * Return to plain success and clear the markers.
 *
 * The markers have to be gone before each test: `crash_once` decides whether to die by whether its
 * marker exists, so one left behind by an earlier test would make the next one silently observe no
 * crash at all and still pass.
 */
export function resetScenario(scenarioFile) {
  for (const engine of ENGINE_SECTIONS) {
    for (const name of ["crashed", "hang-started", "hang-completed"]) {
      rmSync(markerPath(scenarioFile, engine, name), { force: true });
    }
  }
  writeScenario(scenarioFile, defaultScenario());
}

/** Overlay one or more sections onto a fresh default and publish the result. */
export function applyScenario(scenarioFile, overrides) {
  const scenario = defaultScenario();
  for (const [section, value] of Object.entries(overrides)) {
    if (!scenario[section]) throw new Error(`unknown scenario section: ${section}`);
    scenario[section] = { ...scenario[section], ...value };
  }
  writeScenario(scenarioFile, scenario);
  return scenario;
}
