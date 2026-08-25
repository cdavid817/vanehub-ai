"""Launch the worker exactly the way the host does, as a real subprocess.

`python -u -m vane_local_media_worker` is the only launch form production uses, and it is the form
in which `__name__` is `"__main__"`. A source assertion alone would not have caught the defect this
covers, because the code read correctly in every other context; only running it this way did.

Every engine is exercised, so the same mistake cannot come back for one of the three. The
third-party inference packages are the repository's test-only stand-ins, so nothing here downloads
a model, opens a socket, or depends on a real engine being installed.

Every read is bounded and the child is killed and reaped on the way out. A bare blocking
`readline` would hang forever if the worker died before its hello frame -- which is exactly the
failure this file exists to detect. Batching the request and the shutdown into one write is not an
option either: shutdown cancels in-flight work, so the response would be lost by design.
"""

from __future__ import annotations

import json
import os
import queue
import subprocess
import sys
import tempfile
import threading
import unittest
from pathlib import Path

BRIDGE_ROOT = Path(__file__).resolve().parents[1]
REPOSITORY_ROOT = BRIDGE_ROOT.parents[2]
FIXTURE_ROOT = REPOSITORY_ROOT / "tests" / "desktop" / "fixtures" / "local-media-python"

#: Generous enough for a cold interpreter on a loaded CI runner, short enough that a hang fails the
#: suite rather than stalling it.
TIMEOUT_SECONDS = 60

ENGINES = ("paddleocr", "faster-whisper", "sherpa-onnx")


def worker_environment(media_root: Path) -> dict:
    """The production shape: the bridge first, the stand-in packages after it."""

    keep = ("PATH", "SYSTEMROOT", "SYSTEMDRIVE", "WINDIR", "COMSPEC", "PATHEXT", "TMP", "TEMP")
    environment = {name: os.environ[name] for name in keep if name in os.environ}
    environment.update(
        {
            "PYTHONPATH": os.pathsep.join([str(BRIDGE_ROOT), str(FIXTURE_ROOT)]),
            "PYTHONUNBUFFERED": "1",
            "PYTHONDONTWRITEBYTECODE": "1",
            "HF_HUB_OFFLINE": "1",
            "TRANSFORMERS_OFFLINE": "1",
            "VANEHUB_LOCAL_MEDIA_ROOT": str(media_root),
        }
    )
    return environment


class ModuleEntryTest(unittest.TestCase):
    @unittest.skipUnless(FIXTURE_ROOT.is_dir(), "the desktop Python fixtures are unavailable")
    def test_every_engine_starts_and_answers_over_the_real_protocol(self):
        for engine in ENGINES:
            with self.subTest(engine=engine):
                self._round_trip(engine)

    def _round_trip(self, engine: str) -> None:
        with tempfile.TemporaryDirectory(prefix="vanehub-module-entry-") as media_root:
            process = subprocess.Popen(
                [sys.executable, "-u", "-m", "vane_local_media_worker",
                 "--engine", engine, "--protocol", "1"],
                stdin=subprocess.PIPE,
                stdout=subprocess.PIPE,
                stderr=subprocess.PIPE,
                env=worker_environment(Path(media_root)),
                cwd=media_root,
                text=True,
            )
            lines: "queue.Queue[str | None]" = queue.Queue()

            def pump() -> None:
                for line in process.stdout:  # type: ignore[union-attr]
                    lines.put(line)
                lines.put(None)

            reader = threading.Thread(target=pump, daemon=True)
            reader.start()

            def next_frame(what: str) -> dict:
                try:
                    line = lines.get(timeout=TIMEOUT_SECONDS)
                except queue.Empty:
                    raise AssertionError(
                        f"{engine} produced no {what} within {TIMEOUT_SECONDS}s"
                    ) from None
                if line is None:
                    raise AssertionError(f"{engine} exited before its {what}")
                return json.loads(line)

            try:
                greeting = next_frame("hello frame")
                # Exit code 3 is `engine_module_load_failed`; a hello frame proves the engine
                # module resolved, which is the regression this test exists for.
                self.assertEqual(greeting["type"], "hello")
                self.assertEqual(greeting["engine"], engine)
                self.assertEqual(greeting["v"], 1)

                request = {"v": 1, "type": "request", "id": "r1", "method": "probe", "params": {}}
                process.stdin.write(json.dumps(request) + "\n")
                process.stdin.flush()

                response = next_frame("response frame")
                self.assertEqual(response["type"], "response")
                self.assertEqual(response["id"], "r1")
                # A probe against a stand-in package with no configured model may legitimately
                # report a mapped error; what matters is that a real frame came back over the real
                # protocol.
                if not response["ok"]:
                    self.assertRegex(response["error"]["code"], r"^[A-Z_]+$")

                process.stdin.write(json.dumps({"v": 1, "type": "shutdown"}) + "\n")
                process.stdin.flush()
                self.assertEqual(
                    process.wait(timeout=TIMEOUT_SECONDS), 0, f"{engine} exited non-zero"
                )
            finally:
                if process.poll() is None:
                    process.kill()
                    process.wait(timeout=TIMEOUT_SECONDS)
                stderr = process.stderr.read() if process.stderr else ""
                for stream in (process.stdin, process.stdout, process.stderr):
                    if stream and not stream.closed:
                        stream.close()
                reader.join(timeout=5)
            self._assert_stderr_is_safe(engine, stderr)

    def _assert_stderr_is_safe(self, engine: str, stderr: str) -> None:
        # Diagnostics are `key=value` pairs from an allowlist. A traceback or a path here would mean
        # the redaction the host relies on had already failed by the time anything was logged.
        for forbidden in ("Traceback", str(BRIDGE_ROOT), str(FIXTURE_ROOT)):
            self.assertNotIn(forbidden, stderr, f"{engine} stderr carried unsafe content")

    @unittest.skipUnless(FIXTURE_ROOT.is_dir(), "the desktop Python fixtures are unavailable")
    def test_an_unknown_engine_is_refused_before_a_worker_starts(self):
        with tempfile.TemporaryDirectory(prefix="vanehub-module-entry-") as media_root:
            completed = subprocess.run(
                [sys.executable, "-u", "-m", "vane_local_media_worker",
                 "--engine", "whisper.cpp", "--protocol", "1"],
                input="",
                capture_output=True,
                text=True,
                env=worker_environment(Path(media_root)),
                cwd=media_root,
                timeout=TIMEOUT_SECONDS,
                check=False,
            )

        self.assertNotEqual(completed.returncode, 0)
        self.assertEqual(completed.stdout, "")


if __name__ == "__main__":
    unittest.main()
