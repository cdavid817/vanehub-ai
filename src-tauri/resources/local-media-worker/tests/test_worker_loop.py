"""The dispatch loop, driven end to end against stand-in engine modules.

No real engine is imported. Every behaviour under test here -- busy rejection, cancellation while
inference blocks, a crash still producing a frame -- is a property of the loop, and pinning it to a
real model would make these tests a report on the developer's machine instead.
"""

from __future__ import annotations

import io
import json
import threading
import unittest
from typing import Any, Dict, List

from vane_local_media_worker import __main__ as worker
from vane_local_media_worker import errors, protocol


class FakeEngine:
    CAPABILITIES = ["probe", "ocr", "cancel", "shutdown"]

    def __init__(self):
        self.entered = threading.Event()
        self.release = threading.Event()
        self.shutdown_calls = 0
        self.blocking = False
        self.raises: BaseException | None = None
        self.result: Dict[str, Any] = {"pages": []}
        self.observed_cancel: List[bool] = []

    def package_version(self):
        return "9.9.9"

    def probe(self, _params):
        return {"engine": "paddleocr", "ready": True}

    def ocr(self, _params, cancel):
        self.entered.set()
        if self.blocking:
            # Mirrors an engine that polls its cancel token rather than one that ignores it.
            self.release.wait(timeout=5)
        self.observed_cancel.append(cancel.is_set())
        if self.raises is not None:
            raise self.raises
        return self.result

    def shutdown(self):
        self.shutdown_calls += 1


class Harness:
    def __init__(self, engine: FakeEngine):
        self.engine = engine
        self.stream = io.StringIO()
        self.writer = protocol.FrameWriter(self.stream)
        self.runtime = worker.WorkerRuntime("paddleocr", engine, self.writer)

    def send(self, frame: Dict[str, Any]):
        self.runtime.handle(frame)

    def request(self, request_id="r1", method="ocr", params=None):
        self.send(
            {
                "v": 1,
                "type": "request",
                "id": request_id,
                "method": method,
                "params": params or {},
            }
        )

    def frames(self) -> List[Dict[str, Any]]:
        return [json.loads(line) for line in self.stream.getvalue().split("\n") if line]

    def wait_for_frames(self, count: int, timeout: float = 5.0):
        deadline = threading.Event()
        waited = 0.0
        while len(self.frames()) < count and waited < timeout:
            deadline.wait(0.02)
            waited += 0.02
        return self.frames()


class DispatchTest(unittest.TestCase):
    def test_answers_a_request_with_a_response_frame(self):
        harness = Harness(FakeEngine())
        harness.engine.result = {"pages": [{"pageNumber": 1, "text": "hello"}]}

        harness.request()
        frames = harness.wait_for_frames(1)

        self.assertTrue(frames[0]["ok"])
        self.assertEqual(frames[0]["id"], "r1")
        self.assertEqual(frames[0]["result"]["pages"][0]["text"], "hello")

    def test_rejects_a_method_the_engine_does_not_serve(self):
        harness = Harness(FakeEngine())

        harness.request(method="transcribe")
        frames = harness.wait_for_frames(1)

        self.assertFalse(frames[0]["ok"])
        self.assertEqual(frames[0]["error"]["code"], errors.WORKER_PROTOCOL_ERROR)

    def test_serves_probe_for_every_engine(self):
        harness = Harness(FakeEngine())

        harness.request(method="probe")
        frames = harness.wait_for_frames(1)

        self.assertTrue(frames[0]["ok"])
        self.assertTrue(frames[0]["result"]["ready"])

    def test_refuses_a_second_request_while_one_is_running(self):
        engine = FakeEngine()
        engine.blocking = True
        harness = Harness(engine)

        harness.request(request_id="first")
        engine.entered.wait(timeout=5)
        harness.request(request_id="second")

        busy = harness.wait_for_frames(1)[0]
        # One inference at a time is the invariant the host's readiness model depends on; queueing
        # here instead would let a second request wait behind a model load with no visible state.
        self.assertEqual(busy["id"], "second")
        self.assertEqual(busy["error"]["code"], errors.ENGINE_BUSY)
        engine.release.set()
        harness.wait_for_frames(2)

    def test_a_cancel_frame_is_readable_while_inference_blocks(self):
        engine = FakeEngine()
        engine.blocking = True
        harness = Harness(engine)

        harness.request(request_id="r1")
        engine.entered.wait(timeout=5)
        # The whole reason inference runs on a worker thread: the main loop must still be able to
        # read this frame.
        harness.send({"v": 1, "type": "cancel", "id": "r1"})
        engine.release.set()

        frames = harness.wait_for_frames(1)
        self.assertFalse(frames[0]["ok"])
        self.assertEqual(frames[0]["error"]["code"], errors.OPERATION_CANCELLED)
        self.assertEqual(engine.observed_cancel, [True])

    def test_a_cancel_for_another_request_is_ignored(self):
        engine = FakeEngine()
        engine.blocking = True
        harness = Harness(engine)

        harness.request(request_id="r1")
        engine.entered.wait(timeout=5)
        harness.send({"v": 1, "type": "cancel", "id": "someone-else"})
        engine.release.set()

        frames = harness.wait_for_frames(1)
        self.assertTrue(frames[0]["ok"])

    def test_an_engine_crash_still_produces_a_mapped_error_frame(self):
        engine = FakeEngine()
        engine.raises = RuntimeError("cannot open C:/Users/alice/models/rec/inference.pdmodel")
        harness = Harness(engine)

        harness.request()
        frames = harness.wait_for_frames(1)

        self.assertFalse(frames[0]["ok"])
        self.assertEqual(frames[0]["error"]["code"], errors.ENGINE_UNAVAILABLE)
        # The path in the exception message must not reach the wire.
        self.assertNotIn("alice", json.dumps(frames[0]))

    def test_a_crash_releases_the_slot_for_the_next_request(self):
        engine = FakeEngine()
        engine.raises = RuntimeError("boom")
        harness = Harness(engine)

        harness.request(request_id="r1")
        harness.wait_for_frames(1)
        engine.raises = None
        harness.request(request_id="r2")

        frames = harness.wait_for_frames(2)
        # A slot leaked by a crash would make the engine permanently busy.
        self.assertTrue(frames[1]["ok"])

    def test_shutdown_stops_the_loop_and_closes_the_engine(self):
        engine = FakeEngine()
        harness = Harness(engine)

        harness.send({"v": 1, "type": "shutdown"})

        self.assertTrue(harness.runtime.stopped.is_set())
        self.assertEqual(engine.shutdown_calls, 1)

    def test_shutdown_survives_an_engine_that_fails_to_close(self):
        engine = FakeEngine()
        engine.shutdown = lambda: (_ for _ in ()).throw(RuntimeError("stuck"))
        harness = Harness(engine)

        harness.send({"v": 1, "type": "shutdown"})

        # A worker that refused to stop because cleanup failed would have to be killed.
        self.assertTrue(harness.runtime.stopped.is_set())

    def test_shutdown_cancels_the_request_in_flight(self):
        engine = FakeEngine()
        engine.blocking = True
        harness = Harness(engine)

        harness.request(request_id="r1")
        engine.entered.wait(timeout=5)
        harness.send({"v": 1, "type": "shutdown"})
        engine.release.set()

        frames = harness.wait_for_frames(1)
        self.assertEqual(frames[0]["error"]["code"], errors.OPERATION_CANCELLED)


class ArgumentTest(unittest.TestCase):
    def test_rejects_an_unknown_engine(self):
        with self.assertRaises(SystemExit):
            worker._parse_args(["--engine", "whisper.cpp", "--protocol", "1"])

    def test_exits_rather_than_negotiating_a_protocol_version(self):
        # There is no downgrade path: a host expecting v2 framing would misread every v1 frame.
        self.assertEqual(worker.main(["--engine", "paddleocr", "--protocol", "2"]), 2)


class HelloFrameTest(unittest.TestCase):
    def test_carries_the_engine_capabilities_and_package_version(self):
        frame = protocol.hello_frame("paddleocr", ["probe", "ocr"], "3.0.0")

        self.assertEqual(frame["v"], protocol.PROTOCOL_VERSION)
        self.assertEqual(frame["type"], protocol.FRAME_HELLO)
        self.assertEqual(frame["engine"], "paddleocr")
        self.assertEqual(frame["capabilities"], ["probe", "ocr"])
        self.assertEqual(frame["packageVersion"], "3.0.0")

    def test_reports_an_unknown_package_version_as_null_rather_than_guessing(self):
        self.assertIsNone(protocol.hello_frame("paddleocr", [], None)["packageVersion"])


if __name__ == "__main__":
    unittest.main()
