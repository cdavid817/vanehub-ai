"""Launch the worker exactly the way the host does, as a real subprocess.

`python -u -m vane_local_media_worker` is the only launch form production uses, and it is the form
in which `__name__` is `"__main__"`. A source assertion alone would not have caught the defect this
covers, because the code read correctly in every other context; only running it this way did.

Every engine is exercised, so the same mistake cannot come back for one of the three. The
third-party inference packages are the repository's test-only stand-ins, so nothing here downloads
a model, opens a socket, or depends on a real engine being installed.
"""

from __future__ import annotations

import json
import os
import subprocess
import sys
import unittest
from pathlib import Path

BRIDGE_ROOT = Path(__file__).resolve().parents[1]
REPOSITORY_ROOT = BRIDGE_ROOT.parents[2]
FIXTURE_ROOT = REPOSITORY_ROOT / "tests" / "desktop" / "fixtures" / "local-media-python"

#: Generous enough for a cold interpreter on a loaded CI runner, short enough that a hang fails the
#: suite rather than stalling it.
TIMEOUT_SECONDS = 60

ENGINES = [
    ("paddleocr", "probe"),
    ("faster-whisper", "probe"),
    ("sherpa-onnx", "probe"),
]


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
        for engine, method in ENGINES:
            with self.subTest(engine=engine):
                self._round_trip(engine, method)

    def _round_trip(self, engine: str, method: str) -> None:
        media_root = Path(os.environ.get("TEMP") or "/tmp") / f"vanehub-module-entry-{engine}"
        media_root.mkdir(parents=True, exist_ok=True)

        process = subprocess.Popen(
            [sys.executable, "-u", "-m", "vane_local_media_worker",
             "--engine", engine, "--protocol", "1"],
            stdin=subprocess.PIPE,
            stdout=subprocess.PIPE,
            stderr=subprocess.PIPE,
            env=worker_environment(media_root),
            cwd=str(media_root),
            text=True,
        )
        try:
            hello = process.stdout.readline()
            self.assertTrue(hello, f"{engine} produced no hello frame")
            greeting = json.loads(hello)
            # Exit code 3 is `engine_module_load_failed`; a hello frame proves the engine module
            # resolved, which is the regression this test exists for.
            self.assertEqual(greeting["type"], "hello")
            self.assertEqual(greeting["engine"], engine)
            self.assertEqual(greeting["v"], 1)

            request = {"v": 1, "type": "request", "id": "r1", "method": method, "params": {}}
            process.stdin.write(json.dumps(request) + "\n")
            process.stdin.flush()

            response = json.loads(process.stdout.readline())
            self.assertEqual(response["type"], "response")
            self.assertEqual(response["id"], "r1")
            # A probe against a stand-in package with no configured model may legitimately report a
            # mapped error; what matters is that a real frame came back over the real protocol.
            self.assertIn("ok", response)
            if not response["ok"]:
                self.assertRegex(response["error"]["code"], r"^[A-Z_]+$")

            process.stdin.write(json.dumps({"v": 1, "type": "shutdown"}) + "\n")
            process.stdin.flush()
            self.assertEqual(process.wait(timeout=TIMEOUT_SECONDS), 0)
        finally:
            if process.poll() is None:
                process.kill()
                process.wait(timeout=TIMEOUT_SECONDS)
            for stream in (process.stdin, process.stdout, process.stderr):
                if stream and not stream.closed:
                    stream.close()

    @unittest.skipUnless(FIXTURE_ROOT.is_dir(), "the desktop Python fixtures are unavailable")
    def test_an_unknown_engine_is_refused_before_a_worker_starts(self):
        process = subprocess.run(
            [sys.executable, "-u", "-m", "vane_local_media_worker",
             "--engine", "whisper.cpp", "--protocol", "1"],
            input="",
            capture_output=True,
            text=True,
            env=worker_environment(Path(os.environ.get("TEMP") or "/tmp")),
            timeout=TIMEOUT_SECONDS,
        )

        self.assertNotEqual(process.returncode, 0)
        self.assertEqual(process.stdout, "")


if __name__ == "__main__":
    unittest.main()
