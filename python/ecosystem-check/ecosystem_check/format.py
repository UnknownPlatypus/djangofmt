"""
Execution, comparison, and summary of `djangofmt` ecosystem checks.
"""

from __future__ import annotations

import asyncio
import glob
import time
from asyncio import create_subprocess_exec
from collections.abc import Sequence
from enum import StrEnum
from pathlib import Path
from subprocess import PIPE
from typing import TYPE_CHECKING

from unidiff import PatchSet

from ecosystem_check import logger
from ecosystem_check.markdown import format_patchset, markdown_project_section
from ecosystem_check.types import Comparison, Diff, HunkDetail, Result, ToolError

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
        for diff in comparison.diffs:
            total_lines_added += diff.lines_added
            total_lines_removed += diff.lines_removed

            patch_set = PatchSet("\n".join(diff.lines))
            patch_sets.append(patch_set)
            total_files_modified += len(patch_set.modified_files)

            if diff:
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
        if all(not diff for diff in comparison.diffs):
            continue  # Skip empty diffs

        files = len({patch_file.path for patch_file in patch_set.modified_files})
        logger.error(patch_set)
        s = "s" if files != 1 else ""
        title = f"+{patch_set.added} -{patch_set.removed} lines across {files} file{s}"

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
        case FormatComparison.BASE_AND_COMP:
            diffs = [await format_and_format(*args)]
        case FormatComparison.BASE_THEN_COMP:
            diffs = [await format_then_format(*args)]
        case FormatComparison.BASE_THEN_COMP_CONVERGE:
            diffs = await format_then_format_converge(*args)
        case _:
            raise ValueError(f"Unknown format comparison type {format_comparison!r}.")

    return Comparison(diffs=diffs, repo=cloned_repo)


async def format_and_format(
    baseline_executable: Path,
    comparison_executable: Path,
    options: FormatOptions,
    cloned_repo: ClonedRepository,
) -> Diff:
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
    return Diff(await cloned_repo.diff(commit))


async def format_then_format(
    baseline_executable: Path,
    comparison_executable: Path,
    options: FormatOptions,
    cloned_repo: ClonedRepository,
) -> Diff:
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
    return Diff(await cloned_repo.diff(commit))


async def format_then_format_converge(
    baseline_executable: Path,
    comparison_executable: Path,
    options: FormatOptions,
    cloned_repo: ClonedRepository,
) -> list[Diff]:
    """Run format_then_format twice, collecting every intermediary diffs"""
    executables = [baseline_executable, comparison_executable] * 2

    hunk_details: set[HunkDetail] = set()
    for i, executable in enumerate(
        [baseline_executable, comparison_executable] * 2, start=1
    ):
        await format(
            executable=executable.resolve(),
            path=cloned_repo.path,
            repo_fullname=cloned_repo.fullname,
            options=options,
        )
        commit = await cloned_repo.commit(
            message=f"Formatted with {executable.name} - #{i}"
        )
        if i > 2:
            # Skip the 2 first runs that are just setting the baseline
            diff = Diff(await cloned_repo.diff(f"{commit}^", commit))
            if diff:
                hunk_details.update(
                    HunkDetail(
                        path=patch_file.path,
                        start=hunk.source_start,
                        length=hunk.source_length,
                    )
                    for patch_file in PatchSet(diff)
                    for hunk in patch_file
                )

    if not hunk_details:
        return []

    logger.debug(f"Processing hunks {hunk_details}")
    diff_tasks = [
        cloned_repo.diff_for_hunk(hunk_detail, f"HEAD~{len(executables)}..HEAD")
        for hunk_detail in hunk_details
    ]
    diff_results = await asyncio.gather(*diff_tasks)
    return [Diff(result) for result in diff_results if result]


async def format(
    *,
    executable: Path,
    path: Path,
    repo_fullname: str,
    options: FormatOptions,
) -> Sequence[str]:
    """Run the given djangofmt binary against the specified path."""
    args = options.to_args(executable_name=executable.name)
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


class FormatComparison(StrEnum):
    # Run baseline executable then reset and run comparison executable.
    # Checks changes in behavior when formatting "unformatted" code
    BASE_AND_COMP = "base-and-comp"

    # Run baseline executable then comparison executable.
    # Checks for changes in behavior when formatting previously "formatted" code
    BASE_THEN_COMP = "base-then-comp"

    # Run baseline executable then comparison executable.
    # Do that multiple time to ensure it converges.
    BASE_THEN_COMP_CONVERGE = "base-then-comp-converge"
