"""Vendor-compatibility classification, against fake packages rather than real engines.

The real engines prove the behaviour once, on a machine that has them. These prove the mapping
stays correct everywhere else -- including on CI, which has none of them installed.
"""

from __future__ import annotations

import os
import sys
import unittest
from unittest import mock

sys.path.insert(0, os.path.dirname(os.path.dirname(os.path.abspath(__file__))))

from vane_local_media_worker import errors, paddle_ocr_engine, privacy  # noqa: E402

NON_ASCII_DIR = os.path.join("C:" + os.sep, "vanehub qualification 本地媒体", "models", "det")
ASCII_DIR = os.path.join("C:" + os.sep, "vanehub", "models", "det")

# The exact text paddlepaddle 3.3.1 raises when its accelerated executor meets an operator it has
# no runtime conversion for. Reproduced verbatim so the classifier is tested against the shape it
# will actually see rather than against a paraphrase.
ONEDNN_MESSAGE = (
    "(Unimplemented) ConvertPirAttribute2RuntimeAttribute not support "
    "[pir::ArrayAttribute<pir::DoubleAttribute>] "
    "(at ..\\paddle\\fluid\\framework\\new_executor\\instruction\\onednn\\onednn_instruction.cc:118)"
)

# What a native loader that could not open the file reports: the file is present, the read yields
# nothing, and the JSON parser complains about an empty document.
EMPTY_PARSE_MESSAGE = (
    "[json.exception.parse_error.101] parse error at line 1, column 1: "
    "attempting to parse an empty input; check that your input string or stream contains the "
    "expected JSON"
)


class PathShapeTest(unittest.TestCase):
    def test_reports_spaces_and_non_ascii_independently(self):
        self.assertEqual(
            privacy.path_shape("C:/plain/models"),
            {"containsSpaces": False, "containsNonAscii": False},
        )
        self.assertEqual(
            privacy.path_shape("C:/with space/models"),
            {"containsSpaces": True, "containsNonAscii": False},
        )
        self.assertEqual(
            privacy.path_shape("C:/本地/models"),
            {"containsSpaces": False, "containsNonAscii": True},
        )

    def test_an_absent_path_has_no_shape(self):
        self.assertEqual(
            privacy.path_shape(None), {"containsSpaces": False, "containsNonAscii": False}
        )


class AccelerationClassificationTest(unittest.TestCase):
    def test_an_unimplemented_operator_becomes_the_acceleration_code(self):
        failure = errors.classify_vendor_exception(
            NotImplementedError(ONEDNN_MESSAGE),
            engine="paddleocr",
            path=ASCII_DIR,
            package_version="3.7.0",
            acceleration="library-default",
        )
        self.assertEqual(failure.code, errors.PADDLE_ONEDNN_MODEL_INCOMPATIBLE)
        self.assertEqual(failure.details["remediation"], errors.REMEDIATION_DISABLE_CPU_ACCELERATION)
        self.assertEqual(failure.details["packageVersion"], "3.7.0")

    def test_the_remediation_is_not_offered_to_someone_who_already_took_it(self):
        # Acceleration is already off, so this failure is something else and must not be labelled
        # with a remedy the user has already applied.
        failure = errors.classify_vendor_exception(
            NotImplementedError(ONEDNN_MESSAGE),
            engine="paddleocr",
            path=ASCII_DIR,
            acceleration="disabled",
        )
        self.assertEqual(failure.code, errors.ENGINE_UNAVAILABLE)


class PathEncodingClassificationTest(unittest.TestCase):
    def test_an_empty_parse_on_a_non_ascii_path_is_an_encoding_failure(self):
        failure = errors.classify_vendor_exception(
            RuntimeError(EMPTY_PARSE_MESSAGE),
            engine="paddleocr",
            field="textDetectionModelDir",
            path=NON_ASCII_DIR,
            package_version="3.7.0",
        )
        self.assertEqual(failure.code, errors.MODEL_PATH_ENCODING_UNSUPPORTED)
        self.assertEqual(failure.details["field"], "textDetectionModelDir")
        self.assertTrue(failure.details["containsNonAscii"])
        self.assertTrue(failure.details["containsSpaces"])
        self.assertEqual(failure.details["remediation"], errors.REMEDIATION_RELOCATE_TO_ASCII_PATH)

    def test_the_same_failure_on_an_ascii_path_is_not_blamed_on_encoding(self):
        # A corrupt model produces the identical message. Claiming an encoding problem would send
        # the user to move files that are already where they should be.
        failure = errors.classify_vendor_exception(
            RuntimeError(EMPTY_PARSE_MESSAGE),
            engine="paddleocr",
            field="textDetectionModelDir",
            path=ASCII_DIR,
        )
        self.assertEqual(failure.code, errors.ENGINE_UNAVAILABLE)


class ErrorPayloadTest(unittest.TestCase):
    def test_no_compatibility_payload_carries_a_path_or_a_message(self):
        failures = [
            errors.classify_vendor_exception(
                NotImplementedError(ONEDNN_MESSAGE),
                engine="paddleocr",
                path=NON_ASCII_DIR,
                acceleration="enabled",
            ),
            errors.classify_vendor_exception(
                RuntimeError(EMPTY_PARSE_MESSAGE),
                engine="paddleocr",
                field="textRecognitionModelDir",
                path=NON_ASCII_DIR,
            ),
            errors.path_encoding_error(
                errors.TTS_DATA_PATH_ENCODING_UNSUPPORTED,
                engine="sherpa-onnx",
                field="dataDir",
                path=NON_ASCII_DIR,
            ),
        ]
        for failure in failures:
            rendered = repr(failure.details)
            self.assertNotIn("本地媒体", rendered)
            self.assertNotIn("vanehub qualification", rendered)
            self.assertNotIn("onednn", rendered.lower())
            self.assertNotIn("parse_error", rendered)
            for key in failure.details:
                self.assertIn(key, errors._ALLOWED_DETAIL_KEYS, key)  # noqa: SLF001

    def test_every_new_code_has_a_message_key(self):
        for code in (
            errors.PADDLE_ONEDNN_MODEL_INCOMPATIBLE,
            errors.MODEL_PATH_ENCODING_UNSUPPORTED,
            errors.TTS_DATA_PATH_ENCODING_UNSUPPORTED,
            errors.TTS_PHONEMIZER_DATA_UNAVAILABLE,
        ):
            self.assertIn(code, errors._MESSAGE_KEYS, code)  # noqa: SLF001


class AccelerationMappingTest(unittest.TestCase):
    """`enable_mkldnn` has to reach the constructor, which is what configures every pipeline stage.

    A process-wide `FLAGS_use_mkldnn` was measured on a real host and does nothing, because PaddleX
    builds its runners from its own pipeline configuration.
    """

    def _captured_kwargs(self, acceleration: str) -> dict:
        captured: dict = {}

        class FakePaddleOCR:
            def __init__(self, **kwargs):
                captured.update(kwargs)

        module = mock.MagicMock()
        module.PaddleOCR = FakePaddleOCR
        with mock.patch.dict(sys.modules, {"paddleocr": module}):
            paddle_ocr_engine._engine_cache.clear()  # noqa: SLF001
            paddle_ocr_engine._build_engine(  # noqa: SLF001
                {
                    "paddleXConfigPath": None,
                    "textDetectionModelDir": self.detection,
                    "textRecognitionModelDir": self.recognition,
                    "textLineOrientationModelDir": None,
                    "language": "en",
                    "device": "cpu",
                    "cpuAcceleration": acceleration,
                }
            )
        return captured

    def setUp(self):
        self.directory = os.path.dirname(os.path.abspath(__file__))
        self.detection = self.directory
        self.recognition = self.directory

    def test_disabled_reaches_the_constructor(self):
        self.assertIs(self._captured_kwargs("disabled")["enable_mkldnn"], False)

    def test_enabled_reaches_the_constructor(self):
        self.assertIs(self._captured_kwargs("enabled")["enable_mkldnn"], True)

    def test_library_default_passes_no_argument_at_all(self):
        # Not `enable_mkldnn=True`: "the user has not chosen" is a different statement, and passing
        # a value would pin a default that belongs to PaddleOCR.
        self.assertNotIn("enable_mkldnn", self._captured_kwargs("library-default"))

    def test_an_absent_mode_behaves_as_the_library_default(self):
        captured: dict = {}

        class FakePaddleOCR:
            def __init__(self, **kwargs):
                captured.update(kwargs)

        module = mock.MagicMock()
        module.PaddleOCR = FakePaddleOCR
        with mock.patch.dict(sys.modules, {"paddleocr": module}):
            paddle_ocr_engine._engine_cache.clear()  # noqa: SLF001
            paddle_ocr_engine._build_engine(  # noqa: SLF001
                {
                    "paddleXConfigPath": None,
                    "textDetectionModelDir": self.detection,
                    "textRecognitionModelDir": self.recognition,
                    "textLineOrientationModelDir": None,
                    "language": "en",
                    "device": "cpu",
                }
            )
        self.assertNotIn("enable_mkldnn", captured)


WORKER_PACKAGE = os.path.join(
    os.path.dirname(os.path.dirname(os.path.abspath(__file__))), "vane_local_media_worker"
)


def _worker_sources() -> dict:
    """Worker modules with comment lines removed.

    Each rule below is explained in a comment beside the code it governs, and a scan that reads
    comments fails on its own explanation -- which is exactly what the first version of this did.
    """

    sources = {}
    for name in sorted(os.listdir(WORKER_PACKAGE)):
        if not name.endswith(".py"):
            continue
        with open(os.path.join(WORKER_PACKAGE, name), encoding="utf-8") as handle:
            code = [line for line in handle.read().split("\n") if not line.lstrip().startswith("#")]
        sources[name] = "\n".join(code)
    return sources


class NoRelocationTest(unittest.TestCase):
    """The application does not move a user's models to make an engine happy.

    Every one of these would "work": copying the model to a temporary ASCII directory, junctioning
    to it, or asking Windows for its 8.3 short name. All three do something to files the user chose
    without being asked, and the short-name trick fails silently on volumes where 8.3 is disabled.
    """

    FORBIDDEN = (
        "shutil.copy",
        "shutil.copytree",
        "shutil.move",
        "os.rename",
        "os.replace",
        "os.link",
        "os.symlink",
        "GetShortPathName",
        "mklink",
    )

    def test_no_worker_module_relocates_a_model(self):
        for name, source in _worker_sources().items():
            for call in self.FORBIDDEN:
                self.assertNotIn(call, source, f"{name} relocates model files with {call}")


class NoSilentFallbackTest(unittest.TestCase):
    def test_the_acceleration_mode_is_read_once_and_never_retried(self):
        source = _worker_sources()["paddle_ocr_engine.py"]
        # One read for the constructor and one for classification. A third would most likely be a
        # retry loop, which is exactly the silent fallback this design refuses.
        self.assertLessEqual(source.count('"cpuAcceleration"'), 2)
        self.assertNotIn("for acceleration in", source)
        self.assertNotIn("except Exception:\n        return _build_engine", source)

    def test_no_engine_module_sets_a_process_wide_acceleration_flag(self):
        # Measured on a real host: PaddleX builds its runners from its own pipeline configuration
        # and never reads this, so setting it would look right and do nothing.
        for name, source in _worker_sources().items():
            self.assertNotIn("FLAGS_use_mkldnn", source, f"{name} sets a process-wide flag")


if __name__ == "__main__":
    unittest.main()
