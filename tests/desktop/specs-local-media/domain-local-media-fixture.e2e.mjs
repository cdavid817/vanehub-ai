import assert from "node:assert/strict";
import { existsSync, readFileSync, readdirSync } from "node:fs";
import path from "node:path";
import process from "node:process";
import { setTimeout as sleep } from "node:timers/promises";

import { applyScenario, markerPath, resetScenario } from "../helpers/local-media-scenario.mjs";
import { readLocalMediaFixture } from "../wdio-local-media-fixture.mjs";

const fixture = readLocalMediaFixture(process.env.VANEHUB_DESKTOP_RESULT_DIR);
const mediaRoot = path.join(process.env.VANEHUB_APP_DATA_DIR, "local-media");
const logDir = path.join(process.env.VANEHUB_APP_DATA_DIR, "logs");

/**
 * Distinctive markers rather than plausible content.
 *
 * Every one of these is content the user would consider theirs, and the privacy test at the end
 * greps the unified log for them. A transcript reading "hello" would be indistinguishable from
 * coincidence; these strings can only be there because something wrote them.
 */
const OCR_LINE = "VANEHUB-OCR-LINE-9f2c";
const TRANSCRIPT = "VANEHUB-TRANSCRIPT-9f2c";
const SPOKEN = "VANEHUB-SPOKEN-9f2c";

const SCOPE = "composer-fixture-scope";
const OTHER_SCOPE = "composer-other-scope";

const invoke = (fn, ...args) => globalThis.browser.tauri.execute(fn, ...args);

/** Call a command and let a rejection propagate as a test failure. */
const call = (command, request) =>
  invoke(
    ({ core }, input) => core.invoke(input.command, input.request ? { request: input.request } : {}),
    { command, request: request ?? null },
  );

/**
 * Call a command expecting it to be refused, and return the text it was refused with.
 *
 * An unexpected success comes back as a string too, so the assertion that reads it reports what
 * actually happened instead of failing on a null it did not expect.
 */
const refusal = (command, request) =>
  invoke(
    async ({ core }, input) => {
      try {
        const result = await core.invoke(input.command, input.request ? { request: input.request } : {});
        return `UNEXPECTED_SUCCESS ${JSON.stringify(result)}`;
      } catch (error) {
        return String(error);
      }
    },
    { command, request: request ?? null },
  );

/** One read of an operation: its terminal outcome, or `null` while it is still running. */
const pollOnce = (operationId) =>
  invoke(
    async ({ core }, id) => {
      try {
        const result = await core.invoke("get_local_media_operation_result", {
          request: { operationId: id },
        });
        return result ? { status: "succeeded", result } : null;
      } catch (error) {
        return { status: "failed", error: String(error) };
      }
    },
    operationId,
  );

/** Poll one operation to its terminal state, the same way the composer's own poller does. */
async function settle(operationId, timeout = 90_000) {
  let outcome = null;
  await globalThis.browser.waitUntil(
    async () => {
      outcome = await pollOnce(operationId);
      return outcome !== null;
    },
    { timeout, interval: 250, timeoutMsg: `operation ${operationId} never settled` },
  );
  return outcome;
}

async function succeeds(operationId, kind) {
  const outcome = await settle(operationId);
  assert.equal(outcome.status, "succeeded", `expected success, got ${outcome.error}`);
  assert.equal(outcome.result.kind, kind);
  return outcome.result.result;
}

async function fails(operationId) {
  const outcome = await settle(operationId);
  assert.equal(outcome.status, "failed", `expected a failure, got ${JSON.stringify(outcome.result)}`);
  return outcome.error;
}

/** Configure all three engines against this run's interpreter and placeholder models. */
async function configure() {
  const current = await call("get_local_media_profile");
  return call("save_local_media_profile", {
    profile: {
      ...current,
      enabled: true,
      ocr: {
        ...current.ocr,
        enabled: true,
        pythonExecutable: fixture.pythonExecutable,
        textDetectionModelDir: fixture.ocrDetectionModelDir,
        textRecognitionModelDir: fixture.ocrRecognitionModelDir,
        language: "en",
      },
      stt: {
        ...current.stt,
        enabled: true,
        pythonExecutable: fixture.pythonExecutable,
        modelDirectory: fixture.sttModelDir,
      },
      tts: {
        ...current.tts,
        enabled: true,
        pythonExecutable: fixture.pythonExecutable,
        modelKind: "vits",
        modelPath: fixture.ttsModelPath,
        tokensPath: fixture.ttsTokensPath,
      },
    },
    expectedRevision: current.revision,
  });
}

/** Stage the "picked" file through the real admission path and start recognition. */
async function startOcr() {
  const source = await call("fixture_local_media_ocr_source");
  const staged = await call("stage_local_media_ocr_source", { path: source });
  const handle = await call("start_local_media_ocr", {
    stagedInputId: staged.stagedInputId,
    composerScopeId: SCOPE,
  });
  return { staged, handle };
}

const entries = (directory) => (existsSync(directory) ? readdirSync(directory) : []);

/**
 * The deterministic local-media layer.
 *
 * Everything between the command and the engine is production code: the command registry, the
 * application service, the operation store, the temp store, the supervisor, the process launcher,
 * the JSON Lines transport, and every cancel, grace, crash and restart rule. A real
 * `vane_local_media_worker` runs in a real child process against a real Python interpreter. What is
 * substituted is only what a headless runner cannot supply -- a microphone, a speaker, a device
 * list, a human choosing a file, and the three inference libraries themselves.
 *
 * Nothing here is evidence that PaddleOCR, faster-whisper or sherpa-onnx work, that a microphone
 * records, or that a speaker plays.
 */
globalThis.describe("VaneHub AI desktop local media, driven deterministically", () => {
  globalThis.before(async () => {
    const root = await globalThis.$("#root");
    await root.waitForExist({ timeout: 120_000 });
    await globalThis.browser.waitUntil(
      async () => (await root.getAttribute("data-vanehub-bootstrap")) === "ready",
      { timeout: 120_000, timeoutMsg: "React bootstrap did not become ready." },
    );
    await configure();
  });

  globalThis.beforeEach(() => {
    resetScenario(fixture.scenarioFile);
  });

  globalThis.it("recognizes a staged image end to end through the real worker", async () => {
    applyScenario(fixture.scenarioFile, { paddleocr: { lines: [OCR_LINE], pages: 1 } });
    const { staged, handle } = await startOcr();

    const result = await succeeds(handle.operationId, "ocr");

    assert.equal(staged.mediaType, "image");
    assert.equal(result.plainText, OCR_LINE);
    assert.equal(result.characterCount, OCR_LINE.length);
    assert.equal(result.pages.length, 1);
    assert.equal(result.pages[0].lineCount, 1);
    assert.equal(result.provenance.engine, "paddleocr");
    // The fixture package's own version, which can only be here if the real engine wrapper
    // imported it and the real protocol carried the answer back.
    assert.equal(result.provenance.engineVersion, "3.0.0-vanehub-fixture");
    assert.equal(result.provenance.language, "en");
    // Provenance names the model shape, never a directory the user configured.
    assert.doesNotMatch(String(result.provenance.modelIdentity), /[/\\]/);
  });

  globalThis.it("separates an empty page from an engine failure", async () => {
    applyScenario(fixture.scenarioFile, { paddleocr: { behaviour: "empty" } });
    const empty = await startOcr();
    const recognized = await succeeds(empty.handle.operationId, "ocr");

    // No recognized text is information, not an error: the operation succeeded and the composer
    // decides what to say about an empty result.
    assert.equal(recognized.plainText, "");
    assert.equal(recognized.characterCount, 0);

    applyScenario(fixture.scenarioFile, {
      paddleocr: { behaviour: "engine_error", message: "fixture refused to recognize" },
    });
    const broken = await startOcr();
    const error = await fails(broken.handle.operationId);

    // The library raised a plain `RuntimeError`; the worker classified it and the host kept the
    // classification rather than inventing a nearer-sounding code.
    assert.match(error, /ENGINE_UNAVAILABLE/);
    // The engine's own message is not the user's error text, and must not leak through as one.
    assert.doesNotMatch(error, /fixture refused to recognize/);
  });

  globalThis.it("captures, writes a real WAV, and transcribes it", async () => {
    applyScenario(fixture.scenarioFile, { "faster-whisper": { text: TRANSCRIPT } });
    const recording = await call("start_microphone_recording", { composerScopeId: SCOPE });
    assert.match(recording.recordingId, /^lmr-[0-9a-f]{32}$/);

    const handle = await call("stop_recording_and_transcribe", {
      recordingId: recording.recordingId,
      composerScopeId: SCOPE,
    });
    const result = await succeeds(handle.operationId, "stt");

    // The transcript came back through the protocol, and the WAV the host wrote was genuinely
    // opened: the fixture library refuses anything that is not 16-bit mono PCM.
    assert.equal(result.text, TRANSCRIPT);
    assert.equal(result.limitReached, false);
    assert.ok(result.durationMs > 0, "the reported duration came from the file, not a constant");
    assert.equal(result.provenance.engine, "faster-whisper");
    assert.equal(result.provenance.engineVersion, "1.0.0-vanehub-fixture");
  });

  globalThis.it("refuses a recording to a composer that does not own it, and cancels for the one that does", async () => {
    const recording = await call("start_microphone_recording", { composerScopeId: SCOPE });

    const foreign = await refusal("cancel_microphone_recording", {
      recordingId: recording.recordingId,
      composerScopeId: OTHER_SCOPE,
    });
    const foreignTranscribe = await refusal("stop_recording_and_transcribe", {
      recordingId: recording.recordingId,
      composerScopeId: OTHER_SCOPE,
    });

    // Scope is what keeps one composer from ending another's capture; without it a second window
    // could stop a recording it never started.
    assert.match(foreign, /RECORDING_NOT_FOUND/);
    assert.match(foreignTranscribe, /RECORDING_NOT_FOUND/);

    await call("cancel_microphone_recording", {
      recordingId: recording.recordingId,
      composerScopeId: SCOPE,
    });
    // Cancelling releases the singleton slot rather than leaving the composer unable to record.
    const next = await call("start_microphone_recording", { composerScopeId: SCOPE });
    await call("cancel_microphone_recording", {
      recordingId: next.recordingId,
      composerScopeId: SCOPE,
    });
    assert.notEqual(next.recordingId, recording.recordingId);
  });

  globalThis.it("reports a denied or absent microphone before anything is recorded", async () => {
    applyScenario(fixture.scenarioFile, { capture: { behaviour: "permission_denied" } });
    const denied = await refusal("start_microphone_recording", { composerScopeId: SCOPE });
    assert.match(denied, /MIC_PERMISSION_DENIED/);

    applyScenario(fixture.scenarioFile, { capture: { behaviour: "device_unavailable" } });
    const unavailable = await refusal("start_microphone_recording", { composerScopeId: SCOPE });
    assert.match(unavailable, /MIC_DEVICE_UNAVAILABLE/);

    applyScenario(fixture.scenarioFile, { devices: { behaviour: "no_devices" } });
    const catalog = await call("list_local_media_audio_devices");
    assert.deepEqual(catalog.inputs, []);

    // A failed start must leave the slot free, or the next press would report a phantom recording.
    resetScenario(fixture.scenarioFile);
    const recovered = await call("start_microphone_recording", { composerScopeId: SCOPE });
    await call("cancel_microphone_recording", {
      recordingId: recovered.recordingId,
      composerScopeId: SCOPE,
    });
  });

  globalThis.it("synthesizes the requested text and stops playback on request", async () => {
    const spoken = `${SPOKEN} ${"a".repeat(200)}`;
    const handle = await call("start_local_media_tts", { text: spoken, composerScopeId: SCOPE });
    const output = path.join(mediaRoot, "operations", handle.operationId, "output.wav");

    // The worker writes the WAV before playback begins, so its presence is the earliest moment a
    // stop could land. Asking repeatedly rather than once removes the race between the file
    // appearing and the playback port registering the utterance -- pressing stop twice is
    // something a user does anyway, and the port treats the second press as a no-op.
    await globalThis.browser.waitUntil(() => existsSync(output), {
      timeout: 60_000,
      interval: 100,
      timeoutMsg: "synthesis never produced an output file",
    });
    const startedAt = Date.now();
    let outcome = null;
    await globalThis.browser.waitUntil(
      async () => {
        await call("stop_local_media_playback", {});
        outcome = await pollOnce(handle.operationId);
        return outcome !== null;
      },
      { timeout: 30_000, interval: 150, timeoutMsg: "playback never ended after being stopped" },
    );
    const elapsed = Date.now() - startedAt;

    assert.equal(outcome.status, "succeeded", `expected success, got ${outcome.error}`);
    const result = outcome.result.result;
    assert.equal(outcome.result.kind, "tts");
    assert.match(result.playbackId, /^lmp-[0-9a-f]{32}$/);
    assert.equal(result.sampleRate, 22_050);
    // The generated audio is about four seconds long at the rate the playback port counts frames.
    // Settling well inside that is what shows the stop was honoured rather than waited out.
    assert.ok(elapsed < 3_000, `playback ran for ${elapsed}ms after being stopped`);
    assert.ok(result.durationMs > 0);
  });

  globalThis.it("survives a worker that dies mid-request and serves the next one", async () => {
    applyScenario(fixture.scenarioFile, {
      "faster-whisper": { behaviour: "crash_once", text: TRANSCRIPT },
    });
    const first = await call("start_microphone_recording", { composerScopeId: SCOPE });
    const crashed = await call("stop_recording_and_transcribe", {
      recordingId: first.recordingId,
      composerScopeId: SCOPE,
    });
    const error = await fails(crashed.operationId);

    // A child that exits without answering is a crashed worker, not a mapped engine error.
    assert.match(error, /WORKER_CRASHED/);
    assert.ok(
      existsSync(markerPath(fixture.scenarioFile, "faster-whisper", "crashed")),
      "the fixture never reached the crash it was scripted to perform",
    );

    const second = await call("start_microphone_recording", { composerScopeId: SCOPE });
    const restarted = await call("stop_recording_and_transcribe", {
      recordingId: second.recordingId,
      composerScopeId: SCOPE,
    });

    // The supervisor drops the poisoned session and launches a fresh process for the next request.
    assert.equal((await succeeds(restarted.operationId, "stt")).text, TRANSCRIPT);
  });

  globalThis.it("kills a worker that ignores the cancel and keeps the engine usable", async () => {
    const hangSeconds = 15;
    applyScenario(fixture.scenarioFile, { "sherpa-onnx": { behaviour: "hang", hangSeconds } });
    const handle = await call("start_local_media_tts", {
      text: `${SPOKEN} hang`,
      composerScopeId: SCOPE,
    });

    // Cancel only once the engine is provably inside its sleep. A worker that never launched would
    // otherwise produce exactly the same cancellation and the same absent completion marker.
    const started = markerPath(fixture.scenarioFile, "sherpa-onnx", "hang-started");
    await globalThis.browser.waitUntil(() => existsSync(started), {
      timeout: 60_000,
      interval: 100,
      timeoutMsg: "the worker never reached the scripted hang",
    });
    const hangBeganAt = Date.now();
    await call("cancel_local_media_operation", { operationId: handle.operationId });
    assert.match(await fails(handle.operationId), /OPERATION_CANCELLED/);

    applyScenario(fixture.scenarioFile, { "sherpa-onnx": { behaviour: "success" } });
    const recovered = await call("start_local_media_tts", {
      text: `${SPOKEN} again`,
      composerScopeId: SCOPE,
    });
    assert.ok((await succeeds(recovered.operationId, "tts")).durationMs > 0);

    // Wait past the point the abandoned sleep would have finished. The marker the fixture writes
    // after its sleep is the difference between a process the host killed and one it merely stopped
    // waiting for -- and only the first is what the grace period promises.
    const remaining = hangSeconds * 1_000 - (Date.now() - hangBeganAt) + 4_000;
    if (remaining > 0) await sleep(remaining);
    assert.equal(
      existsSync(markerPath(fixture.scenarioFile, "sherpa-onnx", "hang-completed")),
      false,
      "the hung worker outlived its cancel grace instead of being killed",
    );
  });

  globalThis.it("leaves no recording, staged source, or generated audio behind", async () => {
    // Everything above has settled by now, so anything still here was never cleaned up. Ephemeral
    // media outliving its operation is how a draft the user discarded ends up on disk.
    for (const tier of ["operations", "recordings", "staging"]) {
      assert.deepEqual(entries(path.join(mediaRoot, tier)), [], `${tier} still holds media`);
    }
  });

  globalThis.it("keeps recognized text, transcripts and synthesis input out of the unified log", async () => {
    const logs = entries(logDir).filter((name) => name.endsWith(".log"));
    assert.ok(logs.length > 0, "the unified log produced nothing to check");

    for (const name of logs) {
      const body = readFileSync(path.join(logDir, name), "utf8");
      for (const secret of [OCR_LINE, TRANSCRIPT, SPOKEN]) {
        assert.equal(body.includes(secret), false, `${name} carries user content: ${secret}`);
      }
      // Paths are the other half of the rule: a log naming the picked file describes the user's
      // filesystem even when it says nothing about the content.
      assert.equal(body.includes(fixture.ocrSource), false, `${name} carries the source path`);
      assert.equal(body.includes(mediaRoot), false, `${name} carries the media root path`);
    }
  });

  globalThis.after(async () => {
    // Put the profile back so a later layer against the same data directory inherits nothing.
    const current = await call("get_local_media_profile");
    await call("save_local_media_profile", {
      profile: {
        ...current,
        enabled: false,
        ocr: { ...current.ocr, enabled: false },
        stt: { ...current.stt, enabled: false },
        tts: { ...current.tts, enabled: false },
      },
      expectedRevision: current.revision,
    });
  });
});
