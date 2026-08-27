import assert from "node:assert/strict";
import { mkdtemp, writeFile } from "node:fs/promises";
import { tmpdir } from "node:os";
import { join } from "node:path";

const invoke = (fn, ...args) => globalThis.browser.tauri.execute(fn, ...args);
const blocked = [];

/**
 * Turn OCR on with shape-valid paths that do not exist.
 *
 * Staging refuses outright while the engine is off -- correctly, since copying a file into an
 * operation directory for an engine that cannot run is pure waste -- so the admission checks below
 * are unreachable until the engine is enabled. Save validates shape, not existence, which is what
 * makes a profile like this storable and lets admission be exercised without an installed engine.
 */
async function enableOcr() {
  const current = await invoke(({ core }) => core.invoke("get_local_media_profile"));
  const root = tmpdir();
  return invoke(
    ({ core }, input) => core.invoke("save_local_media_profile", { request: input }),
    {
      profile: {
        ...current,
        enabled: true,
        ocr: {
          ...current.ocr,
          enabled: true,
          pythonExecutable: join(root, "vanehub-absent-python"),
          textDetectionModelDir: join(root, "vanehub-absent-det"),
          textRecognitionModelDir: join(root, "vanehub-absent-rec"),
        },
      },
      expectedRevision: current.revision,
    },
  );
}

/**
 * The real native local-media surface, driven through the real command registry.
 *
 * No Python engine is assumed. Everything asserted here is host-side behaviour that must hold on a
 * machine with nothing installed -- which is the state most developers and every CI runner are in,
 * and the state in which the feature has to fail comprehensibly rather than mysteriously. Anything
 * that genuinely needs an engine is recorded as BLOCKED with the reason, never skipped silently.
 */
globalThis.describe("VaneHub AI desktop local media domain", () => {
  globalThis.before(async () => {
    const root = await globalThis.$("#root");
    await root.waitForExist({ timeout: 120_000 });
    await globalThis.browser.waitUntil(
      async () => (await root.getAttribute("data-vanehub-bootstrap")) === "ready",
      { timeout: 120_000, timeoutMsg: "React bootstrap did not become ready." },
    );
  });

  globalThis.it("starts with a disabled profile rather than refusing to boot without a bridge", async () => {
    const profile = await invoke(({ core }) => core.invoke("get_local_media_profile"));

    // An absent Python bridge or engine must never be a startup failure; the context assembles and
    // reports unavailability at first use.
    assert.equal(profile.profileId, "default");
    assert.equal(profile.enabled, false);
    assert.equal(profile.ocr.enabled, false);
    assert.equal(profile.stt.enabled, false);
    assert.equal(profile.tts.enabled, false);
  });

  globalThis.it("discovers Python environments without mutating the profile or starting a worker", async () => {
    const before = await invoke(({ core }) => core.invoke("get_local_media_profile"));
    const discovery = await invoke(({ core }) => core.invoke("discover_local_media_python_environments"));
    const after = await invoke(({ core }) => core.invoke("get_local_media_profile"));

    assert.equal(discovery.availability, "available");
    assert.ok(Array.isArray(discovery.candidates));
    for (const candidate of discovery.candidates) {
      assert.equal(typeof candidate.executablePath, "string");
      assert.equal(typeof candidate.version.major, "number");
      assert.ok(["compatible", "unsupported"].includes(candidate.compatibility));
    }
    assert.deepEqual(after, before, "discovery changed the saved profile");
  });

  globalThis.it("round-trips a saved profile and bumps its revision", async () => {
    const before = await invoke(({ core }) => core.invoke("get_local_media_profile"));
    const request = {
      profile: { ...before, enabled: true, ocr: { ...before.ocr, language: "en" } },
      expectedRevision: before.revision,
    };
    const saved = await invoke(({ core }, input) => core.invoke("save_local_media_profile", { request: input }), request);

    assert.equal(saved.enabled, true);
    assert.equal(saved.ocr.language, "en");
    assert.ok(saved.revision > before.revision, "the revision did not advance");

    const reread = await invoke(({ core }) => core.invoke("get_local_media_profile"));
    assert.equal(reread.revision, saved.revision);
    assert.equal(reread.ocr.language, "en");
  });

  globalThis.it("rejects a save built on a stale revision instead of overwriting", async () => {
    const current = await invoke(({ core }) => core.invoke("get_local_media_profile"));
    const stale = { profile: { ...current, revision: current.revision - 1 }, expectedRevision: current.revision - 1 };

    const failure = await invoke(
      async ({ core }, input) => {
        try {
          await core.invoke("save_local_media_profile", { request: input });
          return null;
        } catch (error) {
          return String(error);
        }
      },
      stale,
    );

    // Last-write-wins here would silently discard whatever the other window saved.
    assert.ok(failure?.includes("PROFILE_REVISION_CONFLICT"), `unexpected failure: ${failure}`);
  });

  globalThis.it("reports field-level validation without saving anything", async () => {
    const current = await invoke(({ core }) => core.invoke("get_local_media_profile"));
    const candidate = {
      ...current,
      stt: { ...current.stt, enabled: true, pythonExecutable: "python", modelDirectory: "" },
    };
    const issues = await invoke(
      ({ core }, profile) => core.invoke("validate_local_media_profile", { request: { profile } }),
      candidate,
    );

    const fields = issues.map((issue) => issue.field);
    // A bare `python` is not an absolute path, and the model directory is empty; both are the
    // user's to fix, so each has to name its own input.
    assert.ok(fields.includes("pythonExecutable"), `missing pythonExecutable: ${fields}`);
    assert.ok(fields.includes("modelDirectory"), `missing modelDirectory: ${fields}`);
    for (const issue of issues) {
      assert.ok(issue.messageKey.startsWith("localMedia.errors."), `unlocalizable: ${issue.messageKey}`);
    }
  });

  globalThis.it("reports each engine's readiness independently and never as a raw message", async () => {
    const status = await invoke(({ core }) => core.invoke("get_local_media_status"));

    assert.equal(status.nativeAvailable, true);
    assert.deepEqual(
      status.engines.map((engine) => engine.engine).sort(),
      ["ocr", "stt", "tts"],
    );
    for (const engine of status.engines) {
      assert.ok(
        ["disabled", "unconfigured", "checking", "ready", "unavailable", "restartRequired"].includes(
          engine.readiness.state,
        ),
        `unknown readiness for ${engine.engine}: ${engine.readiness.state}`,
      );
      if (engine.readiness.state === "unavailable") {
        assert.match(engine.readiness.code, /^[A-Z_]+$/, "readiness must carry a stable code, not prose");
      }
    }
    const ready = status.engines.filter((engine) => engine.readiness.state === "ready");
    if (ready.length === 0) {
      blocked.push(
        "No local media engine is configured on this host, so probe/OCR/STT/TTS inference was not exercised.",
      );
    }
  });

  globalThis.it("enumerates this host's audio devices without failing when there are none", async () => {
    const catalog = await invoke(({ core }) => core.invoke("list_local_media_audio_devices"));

    assert.ok(Array.isArray(catalog.inputs), "inputs is not an array");
    assert.ok(Array.isArray(catalog.outputs), "outputs is not an array");
    for (const device of [...catalog.inputs, ...catalog.outputs]) {
      assert.equal(typeof device.deviceId, "string");
      assert.equal(typeof device.label, "string");
      assert.equal(typeof device.isDefault, "boolean");
    }
    if (catalog.inputs.length === 0) {
      blocked.push("This host reports no audio input device, so recording was not exercised.");
    }
  });

  globalThis.it("refuses to stage a file whose bytes are not a supported image or PDF", async () => {
    await enableOcr();
    const directory = await mkdtemp(join(tmpdir(), "vanehub-local-media-"));
    const disguised = join(directory, "not-really.png");
    await writeFile(disguised, "this is plain text wearing a png extension", "utf8");

    const failure = await invoke(
      async ({ core }, path) => {
        try {
          await core.invoke("stage_local_media_ocr_source", { request: { path } });
          return null;
        } catch (error) {
          return String(error);
        }
      },
      disguised,
    );

    // Extension is a claim; content is evidence. Trusting the claim would hand the engine a file
    // it cannot read and turn a clear rejection into an opaque worker failure.
    assert.ok(failure?.includes("UNSUPPORTED_MEDIA_TYPE"), `unexpected failure: ${failure}`);
  });

  globalThis.it("refuses to stage a path that does not exist", async () => {
    await enableOcr();
    const missing = join(tmpdir(), "vanehub-local-media-absent", "nothing.png");
    const failure = await invoke(
      async ({ core }, path) => {
        try {
          await core.invoke("stage_local_media_ocr_source", { request: { path } });
          return null;
        } catch (error) {
          return String(error);
        }
      },
      missing,
    );

    assert.ok(failure?.includes("INPUT_NOT_FOUND"), `unexpected failure: ${failure}`);
  });

  globalThis.it("answers a result read for an operation it does not know with null, not an error", async () => {
    const outcome = await invoke(
      async ({ core }) => {
        try {
          return {
            ok: await core.invoke("get_local_media_operation_result", {
              request: { operationId: "op-does-not-exist" },
            }),
          };
        } catch (error) {
          return { error: String(error) };
        }
      },
    );

    // `null` means "not finished, or never existed" -- the composer only ever polls an id a start
    // call handed it, so the unknown case cannot arise in the product and does not earn a code of
    // its own. Expiry is different, and is the one case that does raise `OPERATION_RESULT_EXPIRED`.
    assert.deepEqual(outcome, { ok: null });
  });

  globalThis.it("refuses to cancel a recording that was never started", async () => {
    const failure = await invoke(
      async ({ core }) => {
        try {
          await core.invoke("cancel_microphone_recording", {
            request: { recordingId: "rec-does-not-exist", composerScopeId: "scope-1" },
          });
          return null;
        } catch (error) {
          return String(error);
        }
      },
    );

    assert.ok(failure?.includes("RECORDING_NOT_FOUND"), `unexpected failure: ${failure}`);
  });

  globalThis.it("refuses to stage anything at all while the OCR engine is switched off", async () => {
    const current = await invoke(({ core }) => core.invoke("get_local_media_profile"));
    await invoke(
      ({ core }, input) => core.invoke("save_local_media_profile", { request: input }),
      {
        profile: { ...current, ocr: { ...current.ocr, enabled: false } },
        expectedRevision: current.revision,
      },
    );
    const directory = await mkdtemp(join(tmpdir(), "vanehub-local-media-off-"));
    const sample = join(directory, "page.png");
    await writeFile(sample, "irrelevant", "utf8");

    const failure = await invoke(
      async ({ core }, path) => {
        try {
          await core.invoke("stage_local_media_ocr_source", { request: { path } });
          return null;
        } catch (error) {
          return String(error);
        }
      },
      sample,
    );

    // The gate comes before admission on purpose: copying a file into an operation directory for
    // an engine that cannot run is pure waste, and the user's instruction is "turn it on", not
    // "pick a different file".
    assert.ok(failure?.includes("ENGINE_DISABLED"), `unexpected failure: ${failure}`);
  });

  globalThis.it("refuses synthesis when the engine is not configured, without starting a process", async () => {
    const failure = await invoke(
      async ({ core }) => {
        try {
          await core.invoke("start_local_media_tts", {
            request: { text: "hello", composerScopeId: "scope-1" },
          });
          return null;
        } catch (error) {
          return String(error);
        }
      },
    );

    // Disabled and unconfigured are different instructions to the user, and either is acceptable
    // here; what is not acceptable is a generic failure or a silently-started worker.
    assert.ok(
      failure && /ENGINE_DISABLED|ENGINE_UNCONFIGURED|LOCAL_MEDIA_DISABLED/.test(failure),
      `unexpected failure: ${failure}`,
    );
  });

  globalThis.after(async () => {
    // Leave the host's profile as it was found, so a later layer does not inherit this one's edits.
    const current = await invoke(({ core }) => core.invoke("get_local_media_profile"));
    await invoke(
      ({ core }, input) => core.invoke("save_local_media_profile", { request: input }),
      {
        profile: {
          ...current,
          enabled: false,
          ocr: {
            ...current.ocr,
            enabled: false,
            language: "ch",
            pythonExecutable: "",
            textDetectionModelDir: null,
            textRecognitionModelDir: null,
          },
        },
        expectedRevision: current.revision,
      },
    );
    if (blocked.length > 0) {
      globalThis.console.warn(`BLOCKED on this host:\n  ${blocked.join("\n  ")}`);
    }
  });
});
