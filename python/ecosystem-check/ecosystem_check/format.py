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

from ecosystem_check import logger
from ecosystem_check.markdown import markdown_project_section
from ecosystem_check.projects import Formatter, Profile
from ecosystem_check.types import (
    Comparison,
    Diff,
    HistoriesForHunks,
    HunkDetail,
    Result,
    ToolError,
)

if TYPE_CHECKING:
    from ecosystem_check.projects import (
        ClonedRepository,
        FormatOptions,
        Project,
    )


def add_s(n: int) -> str:
    return "s" if n != 1 else ""


def can_format_project(
    baseline_executable: Path, comparison_executable: Path, target: Project
) -> bool:
    """Skip project if one of the executables is djade and the profile is jinja."""
    return not any(
        executable.name == Formatter.DJADE
        and target.format_options.profile == Profile.JINJA
        for executable in [baseline_executable, comparison_executable]
    )


def markdown_format_result(result: Result) -> str:
    """
    Render a `djangofmt` ecosystem check result as markdown.
    """
    error_count = len(result.errored)
    projects_with_changes = sum(bool(comp.diff) for _, comp in result.completed)
    total_lines_added = sum(comp.diff.lines_added for _, comp in result.completed)
    total_lines_removed = sum(comp.diff.lines_removed for _, comp in result.completed)
    total_files_modified = sum(comp.diff.modified_files for _, comp in result.completed)

    if total_lines_removed == 0 and total_lines_added == 0 and error_count == 0:
        return "\u2705 ecosystem check detected no format changes."

    # Summarize the total changes
    lines: list[str] = []
    if total_lines_added == 0 and total_lines_added == 0:
        # Only errors
        lines.append(
            f"\u2139\ufe0f ecosystem check **encountered format errors**. "
            f"(no format changes; {error_count} project error{add_s(error_count)})"
        )
    else:
        changes = (
            f"+{total_lines_added} -{total_lines_removed} lines "
            f"in {total_files_modified} file{add_s(total_files_modified)} in "
            f"{projects_with_changes} projects"
        )

        if error_count:
            changes += f"; {error_count} project error{add_s(error_count)}"

        unchanged_projects = len(result.completed) - projects_with_changes
        if unchanged_projects:
            changes += (
                f"; {unchanged_projects} project{add_s(unchanged_projects)} unchanged"
            )

        lines.append(
            f"\u2139\ufe0f ecosystem check **detected format changes**. ({changes})"
        )

    lines.append("")

    # Then per-project changes
    for project, comparison in result.completed:
        if not comparison.diff:
            continue  # Skip empty diffs

        files = comparison.diff.modified_files
        title = f"+{comparison.diff.lines_added} -{comparison.diff.lines_removed} lines across {files} file{add_s(files)}"

        lines.extend(
            markdown_project_section(
                title=title,
                content=comparison.diff.format_markdown(repo=comparison.repo),
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
            diff: Diff | HistoriesForHunks = await format_and_format(*args)
        case FormatComparison.BASE_THEN_COMP:
            diff = await format_then_format(*args)
        case FormatComparison.BASE_THEN_COMP_CONVERGE:
            diff = await format_then_format_converge(*args)
        case _:
            raise ValueError(f"Unknown format comparison type {format_comparison!r}.")

    return Comparison(diff=diff, repo=cloned_repo)


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
) -> HistoriesForHunks:
    """Run format_then_format twice, collecting every intermediary diffs"""
    executables = [baseline_executable, comparison_executable] * 2

    hunk_details: set[HunkDetail] = set()
    for i, executable in enumerate(executables, start=1):
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
                    for patch_file in diff.patch_set
                    for hunk in patch_file
                )

    if not hunk_details:
        return HistoriesForHunks()

    logger.debug(f"Processing hunks {hunk_details}")
    diff_tasks = [
        cloned_repo.diff_for_hunk(hunk_detail, f"HEAD~{len(executables)}..HEAD")
        for hunk_detail in hunk_details
    ]
    diff_results = await asyncio.gather(*diff_tasks)
    return HistoriesForHunks(Diff(result) for result in diff_results if result)


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
    ) - set(options.excluded_files(executable.name))
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
