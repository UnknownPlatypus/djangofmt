"""
Execution, comparison, and summary of `djangofmt` ecosystem checks.
"""

from __future__ import annotations

import glob
import time
from asyncio import create_subprocess_exec
from collections.abc import Sequence
from enum import Enum
from pathlib import Path
from subprocess import PIPE
from typing import TYPE_CHECKING

from unidiff import PatchSet

from ecosystem_check import logger
from ecosystem_check.markdown import format_patchset, markdown_project_section
from ecosystem_check.types import Comparison, Diff, Result, ToolError

if TYPE_CHECKING:
    from ecosystem_check.projects import (
        ClonedRepository,
        FormatOptions,
    )


def markdown_format_result(result: Result) -> str:
    """
    Render a `djangofmt` ecosystem check result as markdown.
    """
    lines: list[str] = []
    total_lines_removed = total_lines_added = 0
    total_files_modified = 0
    projects_with_changes = 0
    error_count = len(result.errored)
    patch_sets: list[PatchSet] = []

    for _, comparison in result.completed:
        total_lines_added += comparison.diff.lines_added
        total_lines_removed += comparison.diff.lines_removed

        patch_set = PatchSet("\n".join(comparison.diff.lines))
        patch_sets.append(patch_set)
        total_files_modified += len(patch_set.modified_files)

        if comparison.diff:
            projects_with_changes += 1

    if total_lines_removed == 0 and total_lines_added == 0 and error_count == 0:
        return "\u2705 ecosystem check detected no format changes."

    # Summarize the total changes
    if total_lines_added == 0 and total_lines_added == 0:
        # Only errors
        s = "s" if error_count != 1 else ""
        lines.append(
            f"\u2139\ufe0f ecosystem check **encountered format errors**. "
            f"(no format changes; {error_count} project error{s})"
        )
    else:
        s = "s" if total_files_modified != 1 else ""
        changes = (
            f"+{total_lines_added} -{total_lines_removed} lines "
            f"in {total_files_modified} file{s} in "
            f"{projects_with_changes} projects"
        )

        if error_count:
            s = "s" if error_count != 1 else ""
            changes += f"; {error_count} project error{s}"

        unchanged_projects = len(result.completed) - projects_with_changes
        if unchanged_projects:
            s = "s" if unchanged_projects != 1 else ""
            changes += f"; {unchanged_projects} project{s} unchanged"

        lines.append(
            f"\u2139\ufe0f ecosystem check **detected format changes**. ({changes})"
        )

    lines.append("")

    # Then per-project changes
    for (project, comparison), patch_set in zip(
        result.completed, patch_sets, strict=False
    ):
        if not comparison.diff:
            continue  # Skip empty diffs

        files = len(patch_set.modified_files)
        s = "s" if files != 1 else ""
        title = (
            f"+{comparison.diff.lines_added} -{comparison.diff.lines_removed} "
            f"lines across {files} file{s}"
        )

        lines.extend(
            markdown_project_section(
                title=title,
                content=format_patchset(patch_set, comparison.repo),
                options=project.format_options,
                project=project,
            )
        )

    for project, error in result.errored:
        lines.extend(
            markdown_project_section(
                title="error",
                content=f"```\n{str(error).strip()}\n```",
                options=project.format_options,
                project=project,
            )
        )

    return "\n".join(lines)


async def compare_format(
    baseline_executable: Path,
    comparison_executable: Path,
    options: FormatOptions,
    cloned_repo: ClonedRepository,
    format_comparison: FormatComparison,
) -> Comparison:
    args = (
        baseline_executable,
        comparison_executable,
        options,
        cloned_repo,
    )
    match format_comparison:
        case FormatComparison.base_then_comp:
            coro = format_then_format(*args)
        case FormatComparison.base_and_comp:
            coro = format_and_format(*args)
        case _:
            raise ValueError(f"Unknown format comparison type {format_comparison!r}.")

    diff = await coro
    return Comparison(diff=Diff(diff), repo=cloned_repo)


async def format_then_format(
    baseline_executable: Path,
    comparison_executable: Path,
    options: FormatOptions,
    cloned_repo: ClonedRepository,
) -> Sequence[str]:
    # Run format to get the baseline
    await format(
        executable=baseline_executable.resolve(),
        path=cloned_repo.path,
        repo_fullname=cloned_repo.fullname,
        options=options,
    )

    # Commit the changes
    commit = await cloned_repo.commit(
        message=f"Formatted with baseline {baseline_executable}"
    )

    # Then run format again
    await format(
        executable=comparison_executable.resolve(),
        path=cloned_repo.path,
        repo_fullname=cloned_repo.fullname,
        options=options,
    )

    # Then get the diff from the commit
    diff = await cloned_repo.diff(commit)

    return diff


async def format_and_format(
    baseline_executable: Path,
    comparison_executable: Path,
    options: FormatOptions,
    cloned_repo: ClonedRepository,
) -> Sequence[str]:
    # Run format without diff to get the baseline
    await format(
        executable=baseline_executable.resolve(),
        path=cloned_repo.path,
        repo_fullname=cloned_repo.fullname,
        options=options,
    )

    # Commit the changes
    commit = await cloned_repo.commit(
        message=f"Formatted with baseline {baseline_executable}"
    )
    # Then reset
    await cloned_repo.reset()

    # Then run format again
    await format(
        executable=comparison_executable.resolve(),
        path=cloned_repo.path,
        repo_fullname=cloned_repo.fullname,
        options=options,
    )

    # Then get the diff from the commit
    diff = await cloned_repo.diff(commit)

    return diff


async def format(
    *,
    executable: Path,
    path: Path,
    repo_fullname: str,
    options: FormatOptions,
) -> Sequence[str]:
    """Run the given djangofmt binary against the specified path."""
    args = options.to_args()
    files = set(
        glob.iglob("**/*templates/**/*.html", recursive=True, root_dir=path)
    ) - set(options.exclude)
    logger.debug(
        f"Formatting {repo_fullname} with cmd {executable!r} ({len(files)} files)"
    )
    if options.exclude:
        logger.debug(f"Excluding {options.exclude}")

    start = time.perf_counter()
    proc = await create_subprocess_exec(
        executable.absolute(),
        *args,
        *files,
        stdout=PIPE,
        stderr=PIPE,
        cwd=path,
    )
    result, err = await proc.communicate()
    end = time.perf_counter()

    logger.debug(
        f"Finished formatting {repo_fullname} with {executable} in {end - start:.2f}s"
    )

    if proc.returncode not in [0, 1]:
        raise ToolError(err.decode("utf8"))

    lines = result.decode("utf8").splitlines()
    return lines


class FormatComparison(Enum):
    # Run baseline executable then comparison executable.
    # Checks for changes in behavior when formatting previously "formatted" code
    base_then_comp = "base-then-comp"

    # Run baseline executable then reset and run comparison executable.
    # Checks changes in behavior when formatting "unformatted" code
    base_and_comp = "base-and-comp"
