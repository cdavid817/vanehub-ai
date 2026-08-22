"""Path confinement, detail redaction, and exception classification."""

from __future__ import annotations

import io
import os
import sys
import tempfile
import unittest
from unittest import mock

from vane_local_media_worker import errors, privacy


class AdmittedMediaPathTest(unittest.TestCase):
    def setUp(self):
        self.root = tempfile.mkdtemp(prefix="vane-media-root-")
        self.addCleanup(lambda: None)
        patcher = mock.patch.dict(os.environ, {"VANEHUB_LOCAL_MEDIA_ROOT": self.root})
        patcher.start()
        self.addCleanup(patcher.stop)

    def write(self, name: str) -> str:
        path = os.path.join(self.root, name)
        with open(path, "w", encoding="utf-8") as handle:
            handle.write("x")
        return path

    def test_admits_a_file_inside_the_operation_root(self):
        path = self.write("page.png")

        self.assertEqual(
            privacy.admitted_media_path(path, engine="paddleocr", must_exist=True),
            os.path.realpath(path),
        )

    def test_rejects_a_path_outside_the_operation_root(self):
        outside = tempfile.NamedTemporaryFile(delete=False)
        outside.close()
        self.addCleanup(os.unlink, outside.name)

        with self.assertRaises(errors.WorkerError) as caught:
            privacy.admitted_media_path(outside.name, engine="paddleocr", must_exist=True)

        self.assertEqual(caught.exception.code, errors.INPUT_NOT_FOUND)

    def test_rejects_traversal_that_escapes_the_root_after_resolution(self):
        escaping = os.path.join(self.root, "..", "elsewhere.png")

        with self.assertRaises(errors.WorkerError):
            privacy.admitted_media_path(escaping, engine="paddleocr", must_exist=True)

    def test_rejects_a_relative_path(self):
        with self.assertRaises(errors.WorkerError):
            privacy.admitted_media_path("page.png", engine="paddleocr", must_exist=True)

    def test_rejects_an_empty_or_non_string_path(self):
        for candidate in ["", None, 5]:
            with self.subTest(candidate=candidate):
                with self.assertRaises(errors.WorkerError):
                    privacy.admitted_media_path(candidate, engine="paddleocr", must_exist=True)

    def test_rejects_a_directory_where_a_file_is_required(self):
        with self.assertRaises(errors.WorkerError) as caught:
            privacy.admitted_media_path(self.root, engine="paddleocr", must_exist=True)

        self.assertEqual(caught.exception.code, errors.UNSUPPORTED_MEDIA_TYPE)

    def test_admits_a_not_yet_written_output_path_inside_the_root(self):
        target = os.path.join(self.root, "speech.wav")

        self.assertEqual(
            privacy.admitted_media_path(target, engine="sherpa-onnx", must_exist=False),
            os.path.realpath(target),
        )

    def test_rejects_an_output_path_whose_directory_does_not_exist(self):
        target = os.path.join(self.root, "missing-dir", "speech.wav")

        with self.assertRaises(errors.WorkerError) as caught:
            privacy.admitted_media_path(target, engine="sherpa-onnx", must_exist=False)

        self.assertEqual(caught.exception.code, errors.TEMP_STORAGE_FAILED)


class AdmittedModelPathTest(unittest.TestCase):
    def test_returns_none_for_an_absent_optional_path(self):
        self.assertIsNone(
            privacy.admitted_model_path(
                None, engine="paddleocr", field="lexiconPath", required=False,
                expect_directory=False,
            )
        )

    def test_reports_an_absent_required_path_as_unconfigured_not_missing(self):
        with self.assertRaises(errors.WorkerError) as caught:
            privacy.admitted_model_path(
                "", engine="sherpa-onnx", field="modelPath", required=True, expect_directory=False,
            )

        # "You have not set this" and "what you set is not there" send the user to different places.
        self.assertEqual(caught.exception.code, errors.MODEL_NOT_CONFIGURED)

    def test_reports_a_configured_but_missing_path_as_not_found(self):
        with self.assertRaises(errors.WorkerError) as caught:
            privacy.admitted_model_path(
                os.path.join(tempfile.gettempdir(), "vane-does-not-exist-xyz"),
                engine="faster-whisper", field="modelDirectory", required=True,
                expect_directory=True,
            )

        self.assertEqual(caught.exception.code, errors.MODEL_NOT_FOUND)

    def test_rejects_a_file_where_a_directory_is_expected(self):
        handle = tempfile.NamedTemporaryFile(delete=False)
        handle.close()
        self.addCleanup(os.unlink, handle.name)

        with self.assertRaises(errors.WorkerError):
            privacy.admitted_model_path(
                handle.name, engine="faster-whisper", field="modelDirectory", required=True,
                expect_directory=True,
            )

    def test_rejects_a_relative_model_path(self):
        with self.assertRaises(errors.WorkerError):
            privacy.admitted_model_path(
                "models/rec", engine="paddleocr", field="textRecognitionModelDir", required=True,
                expect_directory=True,
            )

    def test_does_not_require_containment_for_a_user_configured_model(self):
        # Models live wherever the user installed them; only media is confined.
        directory = tempfile.mkdtemp(prefix="vane-model-")

        self.assertEqual(
            privacy.admitted_model_path(
                directory, engine="faster-whisper", field="modelDirectory", required=True,
                expect_directory=True,
            ),
            os.path.realpath(directory),
        )


class SanitizeDetailsTest(unittest.TestCase):
    def test_drops_a_key_that_is_not_on_the_allowlist(self):
        safe = errors.sanitize_details({"path": "/home/user/secret.png", "engine": "paddleocr"})

        self.assertEqual(safe, {"engine": "paddleocr"})

    def test_drops_a_long_string_rather_than_truncating_it(self):
        safe = errors.sanitize_details({"field": "x" * 65})

        # A partially redacted path is still a path.
        self.assertEqual(safe, {})

    def test_drops_a_string_containing_control_characters(self):
        self.assertEqual(errors.sanitize_details({"field": "model\ndir"}), {})

    def test_keeps_bounded_scalars(self):
        safe = errors.sanitize_details({"limit": 20, "actual": 31, "device": "cpu"})

        self.assertEqual(safe, {"limit": 20, "actual": 31, "device": "cpu"})

    def test_drops_a_structured_value_even_under_an_allowed_key(self):
        self.assertEqual(errors.sanitize_details({"field": {"nested": "value"}}), {})


class ClassifyExceptionTest(unittest.TestCase):
    def test_keeps_only_the_exception_class_name(self):
        mapped = errors.classify_exception(
            RuntimeError("failed to load C:/Users/alice/models/rec"), engine="paddleocr"
        )

        wire = mapped.to_wire()
        # A model loader routinely puts the model path into str(exc); discarding the message here
        # rather than at the host boundary is what keeps it out of the log.
        self.assertEqual(wire["safeDetails"]["exceptionType"], "RuntimeError")
        self.assertNotIn("alice", str(wire))

    def test_maps_a_missing_module_to_an_import_failure(self):
        mapped = errors.classify_exception(ModuleNotFoundError("no paddleocr"), engine="paddleocr")

        self.assertEqual(mapped.code, errors.ENGINE_IMPORT_FAILED)

    def test_maps_a_missing_file_to_a_missing_model(self):
        mapped = errors.classify_exception(FileNotFoundError(2, "nope"), engine="faster-whisper")

        self.assertEqual(mapped.code, errors.MODEL_NOT_FOUND)

    def test_maps_a_denied_socket_to_a_blocked_download(self):
        class NetworkAccessDeniedError(OSError):
            pass

        try:
            try:
                raise NetworkAccessDeniedError("denied")
            except NetworkAccessDeniedError as inner:
                raise OSError("hub unreachable") from inner
        except OSError as outer:
            mapped = errors.classify_exception(outer, engine="faster-whisper")

        # This is the error the user has to act on: the model is not present and nothing will fetch
        # it, which is different from "the engine broke".
        self.assertEqual(mapped.code, errors.MODEL_DOWNLOAD_BLOCKED)

    def test_passes_a_worker_error_through_unchanged(self):
        original = errors.WorkerError(errors.NO_TEXT_DETECTED, engine="paddleocr")

        self.assertIs(errors.classify_exception(original, engine="paddleocr"), original)

    def test_marks_only_transient_codes_as_retryable(self):
        self.assertTrue(errors.WorkerError(errors.ENGINE_BUSY).to_wire()["retryable"])
        self.assertFalse(errors.WorkerError(errors.MODEL_NOT_FOUND).to_wire()["retryable"])


class DiagnosticTest(unittest.TestCase):
    def capture(self, engine, event, **fields):
        original = sys.stderr
        sys.stderr = io.StringIO()
        try:
            privacy.diagnostic(engine, event, **fields)
            return sys.stderr.getvalue()
        finally:
            sys.stderr = original

    def test_writes_one_line_of_allowlisted_scalars(self):
        line = self.capture("paddleocr", "request_failed", exceptionType="RuntimeError")

        self.assertEqual(line, "engine=paddleocr event=request_failed exceptionType=RuntimeError\n")

    def test_never_emits_a_path_even_when_one_is_passed(self):
        line = self.capture("paddleocr", "request_failed", path="/home/user/secret.png")

        self.assertNotIn("secret", line)
        self.assertEqual(line, "engine=paddleocr event=request_failed\n")


if __name__ == "__main__":
    unittest.main()
