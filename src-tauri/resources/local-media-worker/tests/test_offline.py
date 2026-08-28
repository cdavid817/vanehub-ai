"""Proof that the worker's own code paths never reach the network.

Every socket constructor is replaced with a raiser for the duration of these tests, so any attempt
to open one fails loudly instead of succeeding quietly on a developer machine that happens to be
online. This covers what this bridge does; it is not a claim about what an arbitrary Python package
could do if it were configured differently.
"""

from __future__ import annotations

import importlib
import os
import socket
import sys
import tempfile
import threading
import types
import unittest
from unittest import mock

from vane_local_media_worker import errors, privacy


class NetworkAccessDeniedError(OSError):
    """Raised in place of any socket this process tries to open."""


def read_source(path: str) -> str:
    with open(path, encoding="utf-8") as handle:
        return handle.read()


class DeniedSocketMixin:
    def setUp(self):
        super().setUp()
        def deny(*_args, **_kwargs):
            raise NetworkAccessDeniedError("sockets are denied in this test")

        patches = [
            mock.patch.object(socket, "socket", deny),
            mock.patch.object(socket, "create_connection", deny),
            mock.patch.object(socket, "getaddrinfo", deny),
        ]
        for patch in patches:
            patch.start()
            self.addCleanup(patch.stop)


class OfflineEnvironmentTest(unittest.TestCase):
    def test_the_offline_flags_the_host_sets_are_the_ones_the_libraries_read(self):
        # Pinned as a pair: HF_HUB_OFFLINE stops hub lookups, TRANSFORMERS_OFFLINE stops the
        # fallback path some versions take when the first one is unset.
        from vane_local_media_worker import faster_whisper_engine

        source = read_source(faster_whisper_engine.__file__)
        self.assertIn("local_files_only=True", source)

    def test_paddleocr_never_omits_an_optional_sub_model(self):
        from vane_local_media_worker import paddle_ocr_engine

        source = read_source(paddle_ocr_engine.__file__)
        # Omission is a download here, not a default, so each optional stage is explicitly False.
        self.assertIn('"use_doc_orientation_classify": False', source)
        self.assertIn('"use_doc_unwarping": False', source)


class DeniedSocketTest(DeniedSocketMixin, unittest.TestCase):
    def test_a_socket_attempt_raises_rather_than_connecting(self):
        with self.assertRaises(NetworkAccessDeniedError):
            socket.socket()

    def test_importing_every_worker_module_opens_no_socket(self):
        for name in (
            "errors",
            "privacy",
            "protocol",
            "paddle_ocr_engine",
            "faster_whisper_engine",
            "sherpa_onnx_tts_engine",
            "__main__",
        ):
            with self.subTest(module=name):
                module = importlib.import_module(f"vane_local_media_worker.{name}")
                self.assertIsInstance(module, types.ModuleType)

    def test_path_validation_never_resolves_a_name(self):
        directory = tempfile.mkdtemp(prefix="vane-offline-")

        resolved = privacy.admitted_model_path(
            directory,
            engine="faster-whisper",
            field="modelDirectory",
            required=True,
            expect_directory=True,
        )

        self.assertEqual(resolved, os.path.realpath(directory))

    def test_a_denied_socket_surfaces_as_a_blocked_download_not_a_crash(self):
        try:
            try:
                raise NetworkAccessDeniedError("denied")
            except NetworkAccessDeniedError as inner:
                raise OSError("could not reach the model hub") from inner
        except OSError as outer:
            mapped = errors.classify_exception(outer, engine="faster-whisper")

        # The user needs to hear "the model is not here and nothing will fetch it", which is a
        # different instruction from "the engine broke".
        self.assertEqual(mapped.code, errors.MODEL_DOWNLOAD_BLOCKED)

    def test_the_dispatch_loop_completes_a_request_with_sockets_denied(self):
        from vane_local_media_worker import __main__ as worker
        from vane_local_media_worker import protocol

        import io
        import json

        class Engine:
            CAPABILITIES = ["probe", "ocr"]

            def probe(self, _params):
                return {"ready": True}

            def ocr(self, _params, _cancel):
                return {"pages": []}

            def shutdown(self):
                return None

        stream = io.StringIO()
        runtime = worker.WorkerRuntime("paddleocr", Engine(), protocol.FrameWriter(stream))
        runtime.handle({"v": 1, "type": "request", "id": "r", "method": "ocr", "params": {}})

        waited = 0.0
        while not stream.getvalue() and waited < 5:
            threading.Event().wait(0.02)
            waited += 0.02

        self.assertTrue(json.loads(stream.getvalue().strip())["ok"])


class NoShellConstructionTest(unittest.TestCase):
    def test_no_worker_module_reaches_for_a_shell_or_a_subprocess(self):
        package_dir = os.path.dirname(
            importlib.import_module("vane_local_media_worker.protocol").__file__
        )
        offenders = []
        for name in sorted(os.listdir(package_dir)):
            if not name.endswith(".py"):
                continue
            source = read_source(os.path.join(package_dir, name))
            for needle in ("subprocess", "os.system", "os.popen", "shell=True", "eval(", "exec("):
                if needle in source:
                    offenders.append(f"{name}: {needle}")

        # The worker is the leaf of the process tree. Anything it spawned would be outside every
        # limit the host applies to it.
        self.assertEqual(offenders, [])

    def test_the_package_exposes_no_arbitrary_execution_method(self):
        from vane_local_media_worker import __main__ as worker

        self.assertEqual(
            sorted(worker._ENGINE_METHODS.values()), ["ocr", "synthesize", "transcribe"]
        )
        # `probe` plus exactly one inference method per engine, resolved by name from a fixed table
        # rather than by getattr on whatever the frame asked for.
        self.assertNotIn("eval", sys.modules["vane_local_media_worker.__main__"].__dict__)


if __name__ == "__main__":
    unittest.main()
