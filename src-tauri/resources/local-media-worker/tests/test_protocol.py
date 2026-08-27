"""Protocol framing, sizing, and stdout ownership."""

from __future__ import annotations

import io
import json
import sys
import threading
import unittest

from vane_local_media_worker import protocol


def read_all(text: str):
    return list(protocol.read_frames(io.StringIO(text)))


def request(**overrides):
    frame = {"v": 1, "type": "request", "id": "r1", "method": "probe", "params": {}}
    frame.update(overrides)
    return json.dumps(frame)


class ReadFramesTest(unittest.TestCase):
    def test_accepts_a_well_formed_request(self):
        frames = read_all(request() + "\n")

        self.assertEqual([frame["method"] for frame in frames], ["probe"])

    def test_skips_blank_lines_rather_than_failing_on_them(self):
        frames = read_all("\n  \n" + request() + "\n\n")

        self.assertEqual(len(frames), 1)

    def test_tolerates_crlf_line_endings(self):
        frames = read_all(request() + "\r\n")

        self.assertEqual(len(frames), 1)

    def test_rejects_a_line_that_is_not_json(self):
        with self.assertRaises(protocol.ProtocolError):
            read_all("not json\n")

    def test_rejects_a_json_value_that_is_not_an_object(self):
        with self.assertRaises(protocol.ProtocolError):
            read_all("[1, 2, 3]\n")

    def test_rejects_another_protocol_version(self):
        # Refusing rather than guessing: a v2 host and a v1 worker disagree about field meanings,
        # and the cheapest place to find that out is the first frame.
        with self.assertRaises(protocol.ProtocolError):
            read_all(request(v=2) + "\n")

    def test_rejects_an_outbound_frame_type_arriving_inbound(self):
        with self.assertRaises(protocol.ProtocolError):
            read_all(request(type="response") + "\n")

    def test_rejects_a_missing_or_oversized_request_id(self):
        for identifier in ["", "x" * 129, None, 7]:
            with self.subTest(identifier=identifier):
                with self.assertRaises(protocol.ProtocolError):
                    read_all(request(id=identifier) + "\n")

    def test_rejects_a_missing_or_oversized_method(self):
        for method in ["", "m" * 65, None, []]:
            with self.subTest(method=method):
                with self.assertRaises(protocol.ProtocolError):
                    read_all(request(method=method) + "\n")

    def test_rejects_params_that_are_not_an_object(self):
        with self.assertRaises(protocol.ProtocolError):
            read_all(request(params=["a"]) + "\n")

    def test_allows_params_to_be_absent(self):
        frames = read_all(json.dumps({"v": 1, "type": "request", "id": "r", "method": "probe"}) + "\n")

        self.assertEqual(len(frames), 1)

    def test_refuses_a_request_frame_over_the_inbound_ceiling(self):
        oversized = request(params={"blob": "x" * (protocol.MAX_REQUEST_FRAME_BYTES + 10)})

        # The bound exists to cap what the worker buffers, so it is measured on the encoded line.
        with self.assertRaises(protocol.ProtocolError):
            read_all(oversized + "\n")

    def test_measures_the_inbound_ceiling_in_bytes_not_characters(self):
        # One multi-byte character per two ASCII characters: a character-based check would admit
        # roughly twice the intended number of bytes.
        payload = "測" * (protocol.MAX_REQUEST_FRAME_BYTES // 2)

        with self.assertRaises(protocol.ProtocolError):
            read_all(request(params={"blob": payload}) + "\n")

    def test_accepts_cancel_and_shutdown(self):
        stream = (
            json.dumps({"v": 1, "type": "cancel", "id": "r1"})
            + "\n"
            + json.dumps({"v": 1, "type": "shutdown"})
            + "\n"
        )

        self.assertEqual([frame["type"] for frame in read_all(stream)], ["cancel", "shutdown"])


class FrameWriterTest(unittest.TestCase):
    def test_writes_one_newline_terminated_json_object(self):
        stream = io.StringIO()

        protocol.FrameWriter(stream).write({"v": 1, "type": "response", "id": "r"})

        self.assertEqual(stream.getvalue().count("\n"), 1)
        self.assertEqual(json.loads(stream.getvalue())["id"], "r")

    def test_keeps_non_ascii_text_unescaped_and_compact(self):
        stream = io.StringIO()

        protocol.FrameWriter(stream).write(protocol.response_frame("r", {"text": "識別"}))

        self.assertIn("識別", stream.getvalue())
        self.assertNotIn(", ", stream.getvalue())

    def test_replaces_an_oversized_response_with_a_protocol_error(self):
        stream = io.StringIO()
        huge = "x" * (protocol.MAX_RESPONSE_FRAME_BYTES + 1)

        protocol.FrameWriter(stream).write(protocol.response_frame("r", {"text": huge}))

        frame = json.loads(stream.getvalue())
        # Silently truncating the text would hand the host a result that looks complete.
        self.assertFalse(frame["ok"])
        self.assertEqual(frame["error"]["code"], "WORKER_PROTOCOL_ERROR")
        self.assertNotIn("x" * 100, stream.getvalue())

    def test_never_interleaves_two_concurrent_writes(self):
        stream = io.StringIO()
        writer = protocol.FrameWriter(stream)
        barrier = threading.Barrier(4)

        def emit(index):
            barrier.wait()
            for _ in range(40):
                writer.write(protocol.response_frame(f"r{index}", {"text": "a" * 200}))

        threads = [threading.Thread(target=emit, args=(index,)) for index in range(4)]
        for thread in threads:
            thread.start()
        # The barrier has exactly as many parties as there are writers; the main thread waits on
        # the joins instead, so that all four start writing at once.
        for thread in threads:
            thread.join(timeout=30)
            self.assertFalse(thread.is_alive())

        lines = [line for line in stream.getvalue().split("\n") if line]
        self.assertEqual(len(lines), 160)
        for line in lines:
            # One interleaved write is one unparseable line, which the host reads as stdout
            # contamination and kills the worker for.
            json.loads(line)


class StdoutGuardTest(unittest.TestCase):
    def test_hands_back_the_real_stdout_and_points_prints_at_stderr(self):
        original_stdout, original_stderr = sys.stdout, sys.stderr
        fake_stdout, fake_stderr = io.StringIO(), io.StringIO()
        sys.stdout, sys.stderr = fake_stdout, fake_stderr
        try:
            stream = protocol.install_stdout_guard()
            print("a library banner")
        finally:
            sys.stdout, sys.stderr = original_stdout, original_stderr

        self.assertIs(stream, fake_stdout)
        # The banner must not be able to reach the frame stream.
        self.assertEqual(fake_stdout.getvalue(), "")
        self.assertIn("a library banner", fake_stderr.getvalue())


if __name__ == "__main__":
    unittest.main()
