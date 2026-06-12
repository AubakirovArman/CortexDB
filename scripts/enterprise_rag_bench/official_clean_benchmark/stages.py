"""Pipeline stage registry for official-clean EnterpriseRAG-Bench runs."""

from __future__ import annotations

import argparse
from pathlib import Path
from typing import Callable

from .stage_answer import answer
from .stage_judge import judge
from .stage_prepare import prepare
from .stage_retrieve import retrieve


def selected_stages(stage: str) -> list[tuple[str, Callable[[argparse.Namespace, dict[str, Path]], None]]]:
    stages: list[tuple[str, Callable[[argparse.Namespace, dict[str, Path]], None]]] = [
        ("prepare", prepare),
        ("retrieve", retrieve),
        ("answer", answer),
        ("judge", judge),
    ]
    if stage == "all":
        return stages
    if stage == "retrieval":
        return stages[:2]
    return [(name, runner) for name, runner in stages if name == stage]
