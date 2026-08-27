# Local media runtime

The `local_media` context provides four composer capabilities — recognizing text in an image or PDF, capturing a screen region for text recognition, hold-to-talk transcription, and reading text aloud — and one native tool path for OnePiece's `ocr_read_text`. All four run entirely on the user's own machine.

## Image picker and screen capture

The image/PDF button opens a normal file picker. The screenshot button instead hides VaneHub, freezes each available display, and opens a full-screen region selector. Drag one rectangle, or cancel with **Esc**, right-click, or the cancel button. The selected PNG enters the same editable OCR review as a picked image; recognized text is never appended or sent automatically.

Screen capture is available only in the desktop client and only when OCR is ready. It is intentionally unavailable in Web mode and in a single-Agent CLI terminal. macOS may require Screen Recording permission; Linux support depends on the active X11 or Wayland compositor. Captured pixels remain local, are not written to logs, and are released when capture succeeds, is cancelled, times out, the session changes, or the application exits.

## What the product does not do

This is the shortest way to understand the feature's shape:

- **It never installs anything.** No Python interpreter, no inference package, no CUDA runtime, and no model is downloaded, bundled, or fetched on first use. The settings page has no download button, because there is nothing behind it.
- **It has no cloud fallback.** An engine that is not configured, or not working, is reported as unavailable. It is never quietly replaced by a hosted service.
- **It bundles no model weights.** What ships is roughly 200 KB of Python source under `src-tauri/resources/local-media-worker/vane_local_media_worker/`. An architecture test fails the build if anything that is not a `.py` file appears in that directory.

## What the user has to provide

Each engine is configured independently in **Settings → Local media**, and each may point at a different Python environment.

| Engine | Package | Also needs |
| --- | --- | --- |
| Text recognition | `paddleocr` (plus `paddlepaddle`) | Either a PaddleX pipeline config, or both a text-detection and a text-recognition model directory |
| Speech transcription | `faster-whisper` | A faster-whisper model directory already on disk |
| Speech synthesis | `sherpa-onnx` | A `.onnx` voice model and its `tokens` file, plus the auxiliary files that model kind requires |

Paths must be absolute. `Path::is_absolute` is platform-dependent by design: a profile carrying another operating system's paths is rejected at validation time rather than failing three layers down inside a model loader.

CUDA is selectable but never supplied. Choosing it requires a matching driver and runtime already installed.

## The worker bridge

One Python process per engine, started lazily on first use and kept alive afterwards. The host and the worker exchange one UTF-8 JSON object per line over stdin/stdout, at protocol version 1.

Several properties of that channel are load-bearing rather than incidental:

- **stdout carries frames and nothing else.** `install_stdout_guard()` points `sys.stdout` at stderr before any engine is imported, so a library that prints a progress banner cannot corrupt the stream. A malformed line is treated as contamination and the worker is terminated.
- **Arguments are passed as an argument vector.** No shell is involved at any point, and no worker module imports `subprocess` — a test enforces both.
- **Dispatch stays on the main thread.** Inference runs on a worker thread so that a `cancel` frame is still readable while a model is loading or running.
- **Frames are bounded.** 1 MiB inbound, 8 MiB outbound, measured on the encoded bytes. An oversized response becomes a protocol error rather than a truncated result that looks complete.
- **One inference at a time.** A second request while one is running is refused with `ENGINE_BUSY` rather than queued behind an invisible model load.

## The offline posture, stated precisely

The worker environment removes proxy variables and sets `HF_HUB_OFFLINE=1` and `TRANSFORMERS_OFFLINE=1`. faster-whisper is opened with `local_files_only=True`, and every optional PaddleOCR sub-model is passed as `False` unless its local directory was configured — for PaddleOCR, omitting a model argument is not a neutral default, it is a download.

**This describes the feature's own code paths.** It is not a claim that the operating system prevents an arbitrary Python package from opening a socket. The bridge's test suite denies every socket constructor and shows that the worker's own paths complete anyway; that is the guarantee being made, and it is the one the settings page states.

## Privacy of the data itself

Audio never enters React state or the Tauri JSON IPC. Capture, WAV encoding, playback, temp files, and worker supervision all stay in Rust. The renderer learns an opaque staged-input id and a display name for a picked file; the host path is used once, at the picker, and never returned.

Logging excludes audio bytes, recognized text, transcripts, synthesis input, full paths, complete protocol frames, and raw tracebacks. Worker diagnostics are built from an allowlist of scalars, and a caught exception contributes only its class name — a model loader routinely puts the model path into `str(exc)`, so the message is discarded in the worker rather than at the host boundary.

## Readiness is per engine

A machine with a working PaddleOCR install and no microphone keeps text recognition usable. Each engine card owns its own switch, readiness state, and check button, and reads nothing from the other two.

Checking runs against the **saved** profile, never the draft: a probe starts a real worker with real model paths, and answering "is this configured correctly?" for text the user has not committed would report readiness for a configuration that does not exist.

Repeated startup failures quarantine the engine's worker slot rather than producing a respawn loop. It stays quarantined until the user probes again or saves a new profile.

## Who owns PaddleOCR

`local_media` is the sole production runtime owner of the PaddleOCR **inference process, model lifecycle, readiness, inference, cancellation, timeouts, results, and resource limits**.

That sentence is deliberately narrow, because one other place in the repository still mentions PaddleOCR and it is worth being exact about what it does. The Extension Capabilities page (`tooling/extensions`) lists PaddleOCR as an **installable external dependency**: it creates a managed Python environment, installs `paddleocr` and `paddlepaddle`, records the installed version, and probes a loopback liveness sidecar. It owns none of the things in the list above, and an architecture test fails the build if it names an inference protocol or reaches into `local_media`.

OnePiece's `ocr_read_text` tool reaches this context through an adapter and never touches PaddleOCR itself. The tool's contract is unchanged (`contractVersion: 1`, artifact-only input), and its readiness is derived from the local-media status rather than from a separate copy of the engine's own health.

Installing the package through the Extensions page and pointing this page's **Python interpreter** field at that managed environment is a supported way to set OCR up. Nothing about it moves ownership: the worker is still started, supervised, bounded, and cancelled here.

## Verifying it

- `npm run local-media:python:test` runs the bridge's test suite. It uses only the Python standard library, and reports `NOT RUN` with a reason — never a pass — when no interpreter is present.
- The Rust suite covers admission, staging, the supervisor, capture, and playback against fakes.
- Web-mode Playwright coverage asserts that the browser build stays visibly and explicably unavailable rather than simulating success.

### What that does and does not establish

The core code paths and the deterministic automated tests are implemented and green. Smoke tests against real PaddleOCR, faster-whisper, and sherpa-onnx models, real microphone and speaker hardware, and verification on target platforms other than the developer's own remain **NOT RUN or BLOCKED**. Treat "the tests pass" as a statement about the code paths, not as a report that the three capabilities have been exercised end to end against real engines. `smoke.py` exists for exactly that gap and reports `NOT RUN` per engine until one is installed.

## Where the design lives

This chapter orients contributors; the authoritative requirements live in the corresponding main specs under `openspec/specs`.
