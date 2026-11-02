"""The evidence pack adapter stub (RF-8).

Gate evidence enters retroforge proposals only as
citations behind this adapter. The Gate Truth evidence
pack schema is not pinned for this line yet, so the
adapter consumes nothing: it refuses by name, and no
proposal in this round carries gate citations. When the
schema is pinned, this module learns to read it and the
citations it returns join tape citations under the same
RF-2 resolution rule; until then, any code path that
reaches for gate evidence fails loudly here instead of
guessing a format.
"""
from __future__ import annotations


class EvidencePackError(ValueError):
    """Gate evidence was requested before its format was pinned."""


def gate_citations(path):
    """Refuse: the evidence pack format is not pinned yet."""
    raise EvidencePackError(
        "the evidence pack format is not pinned yet, "
        "no gate evidence can be cited")
