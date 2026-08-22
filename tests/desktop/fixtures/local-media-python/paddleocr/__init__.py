"""Test-only stand-in for `paddleocr`.

Presents the surface `vane_local_media_worker.paddle_ocr_engine` actually uses: a module version,
a `PaddleOCR` constructor accepting the same keyword arguments, and a `predict` returning the
mapping-shaped page results the real 3.x API produces. Everything downstream of that -- line
extraction, ordering, plain-text derivation, truncation, provenance, error mapping -- is the real
production code.
"""

from __future__ import annotations

import sys
from pathlib import Path
from typing import Any, Dict, List

sys.path.insert(0, str(Path(__file__).resolve().parent.parent))

import _vanehub_scenario as scenario  # noqa: E402

__version__ = "3.0.0-vanehub-fixture"

ENGINE = "paddleocr"

_DEFAULT_LINES = [
    ("VaneHub fixture invoice", 0.987),
    ("Line two of the fixture page", 0.964),
]


class PaddleOCR:
    """Accepts the production keyword set and records it for assertion."""

    def __init__(self, **kwargs: Any) -> None:
        # The production wrapper passes every optional sub-model explicitly as False unless a local
        # directory was configured. Asserting that here keeps the fixture honest about the one
        # behaviour that would otherwise trigger a download against the real library.
        for guard in ("use_doc_orientation_classify", "use_doc_unwarping"):
            if kwargs.get(guard) is not False:
                raise ValueError(f"{guard} must be explicitly disabled, got {kwargs.get(guard)!r}")
        self.kwargs = kwargs

    def predict(self, input: str) -> List[Dict[str, Any]]:  # noqa: A002 - the real API names it so
        entry = scenario.load(ENGINE)
        behaviour = scenario.apply_common(ENGINE, entry)

        if behaviour == scenario.ENGINE_ERROR:
            raise RuntimeError(scenario.failure_message(entry))
        if behaviour == scenario.MALFORMED_RESULT:
            # A shape the production extractor cannot read, so the worker's own guards decide what
            # the host sees rather than the fixture deciding for it.
            return [{"rec_texts": "not-a-list", "rec_scores": None}]
        if behaviour == scenario.EMPTY:
            return [{"rec_texts": [], "rec_scores": [], "rec_polys": []}]

        if not Path(input).is_file():
            raise FileNotFoundError(input)

        lines = entry.get("lines")
        pairs = (
            [(str(item), 0.9) for item in lines]
            if isinstance(lines, list) and lines
            else _DEFAULT_LINES
        )
        pages = int(entry.get("pages") or 1)
        return [
            {
                "rec_texts": [text for text, _ in pairs],
                "rec_scores": [score for _, score in pairs],
                "rec_polys": [[[0, 0], [10, 0], [10, 5], [0, 5]] for _ in pairs],
            }
            for _ in range(pages)
        ]
