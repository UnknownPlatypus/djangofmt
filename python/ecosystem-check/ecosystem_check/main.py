from __future__ import annotations

import asyncio
import dataclasses
import json
from collections.abc import Awaitable
from enum import StrEnum
from pathlib import Path
from typing import Any, TypeVar

from ecosystem_check import logger
from ecosystem_check.format import (
    FormatComparison,
    can_format_project,
    compare_format,
    markdown_format_result,
)
from ecosystem_check.projects import (
    Command,
    Project,
)
from ecosystem_check.types import Comparison, Result, Serializable

T = TypeVar("T")
GITHUB_MAX_COMMENT_LENGTH = 65536


class OutputFormat(StrEnum):
    MARKDOWN = "markdown"
    JSON = "json"


async def main(
    command: Command,
    baseline_executable: Path,
    comparison_executable: Path,
    targets: list[Project],
    project_dir: Path,
    output_format: OutputFormat,
    format_comparison: FormatComparison | None,
    max_parallelism: int = 50,
    raise_on_failure: bool = False,
) -> None:
    logger.debug("Using command %s", command.value)
    logger.debug("Using baseline executable at %s", baseline_executable)
    logger.debug("Using comparison executable at %s", comparison_executable)
    logger.debug("Using checkout_dir directory %s", project_dir)
    if format_comparison:
        logger.debug("Using format comparison type %s", format_comparison.value)
    targets = [
        target
        for target in targets
        if can_format_project(baseline_executable, comparison_executable, target)
    ]
    logger.debug("Checking %s targets", len(targets))

    # Limit parallelism to avoid high memory consumption
    semaphore = asyncio.Semaphore(max_parallelism)

    async def limited_parallelism(coroutine: Awaitable[T]) -> T:
        async with semaphore:
            return await coroutine

    comparisons: list[BaseException | Comparison] = await asyncio.gather(
        *[
            limited_parallelism(
                clone_and_compare(
                    command,
                    baseline_executable,
                    comparison_executable,
                    target,
                    project_dir,
                    format_comparison,
                )
            )
            for target in targets
        ],
        return_exceptions=not raise_on_failure,
    )
    comparisons_by_target = dict(zip(targets, comparisons, strict=True))

    # Split comparisons into errored / completed
    errored: list[tuple[Project, BaseException]] = []
    completed: list[tuple[Project, Comparison]] = []
    for target, comparison in comparisons_by_target.items():
        if isinstance(comparison, BaseException):
            errored.append((target, comparison))
        else:
            completed.append((target, comparison))

    result = Result(completed=completed, errored=errored)

    match output_format:
        case OutputFormat.JSON:
            print(json.dumps(result, indent=4, cls=JSONEncoder))
        case OutputFormat.MARKDOWN:
            match command:
                case Command.format:
                    print(markdown_format_result(result))
                case _:
                    raise ValueError(f"Unknown target command {command}")
        case _:
            raise ValueError(f"Unknown output format {format}")

    return None


async def clone_and_compare(
    command: Command,
    baseline_executable: Path,
    comparison_executable: Path,
    target: Project,
    project_dir: Path,
    format_comparison: FormatComparison | None,
) -> Comparison:
    """Check a specific repository against two versions of djangofmt."""
    assert ":" not in target.repo.owner
    assert ":" not in target.repo.name

    match command:
        case Command.format:
            assert format_comparison is not None
            compare, options, kwargs = (
                compare_format,
                target.format_options,
                {"format_comparison": format_comparison},
            )
        case _:
            raise ValueError(f"Unknown target command {command}")

    checkout_dir = project_dir.joinpath(f"{target.repo.owner}:{target.repo.name}")
    cloned_repo = await target.repo.clone(checkout_dir)

    try:
        return await compare(
            baseline_executable,
            comparison_executable,
            options,
            cloned_repo,
            **kwargs,
        )
    except ExceptionGroup as e:
        raise e.exceptions[0] from e


class JSONEncoder(json.JSONEncoder):
    def default(self, o: object) -> Any:
        if isinstance(o, Serializable):
            return o.jsonable()
        if dataclasses.is_dataclass(o):
            return dataclasses.asdict(o)  # type: ignore[arg-type]
        if isinstance(o, set):
            return tuple(o)
        if isinstance(o, Path):
            return str(o)
        if isinstance(o, Exception):
            return str(o)
        return super().default(o)
