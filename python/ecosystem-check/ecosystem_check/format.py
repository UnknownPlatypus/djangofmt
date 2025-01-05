"""
Execution, comparison, and summary of `ruff format` ecosystem checks.
"""

from __future__ import annotations

import time
from asyncio import create_subprocess_exec
from enum import Enum
from pathlib import Path
from subprocess import PIPE
from typing import TYPE_CHECKING, Sequence

from unidiff import PatchSet

from ecosystem_check import logger
from ecosystem_check.markdown import markdown_project_section
from ecosystem_check.types import Comparison, Diff, Result, ToolError

if TYPE_CHECKING:
    from ecosystem_check.projects import (
        ClonedRepository,
        ConfigOverrides,
        FormatOptions,
    )


def markdown_format_result(result: Result) -> str:
    """
    Render a `ruff format` ecosystem check result as markdown.
    """
    lines: list[str] = []
    total_lines_removed = total_lines_added = 0
    total_files_modified = 0
    projects_with_changes = 0
    error_count = len(result.errored)
    patch_sets: list[PatchSet] = []

    for project, comparison in result.completed:
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
            f"\u2139\ufe0f ecosystem check **encountered format errors**. (no format changes; {error_count} project error{s})"
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
    for (project, comparison), patch_set in zip(result.completed, patch_sets):
        if not comparison.diff:
            continue  # Skip empty diffs

        files = len(patch_set.modified_files)
        s = "s" if files != 1 else ""
        title = f"+{comparison.diff.lines_added} -{comparison.diff.lines_removed} lines across {files} file{s}"

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


def format_patchset(patch_set: PatchSet, repo: ClonedRepository) -> str:
    """
    Convert a patchset to markdown, adding permalinks to the start of each hunk.
    """
    lines: list[str] = []
    for file_patch in patch_set:
        for hunk in file_patch:
            # Note:  When used for `format` checks, the line number is not exact because
            #        we formatted the repository for a baseline; we can't know the exact
            #        line number in the original
            #        source file.
            hunk_link = repo.url_for(file_patch.path, hunk.source_start)
            hunk_lines = str(hunk).splitlines()

            # Add a link before the hunk
            link_title = file_patch.path + "~L" + str(hunk.source_start)
            lines.append(f"<a href='{hunk_link}'>{link_title}</a>")

            # Wrap the contents of the hunk in a diff code block
            lines.append("```diff")
            lines.extend(hunk_lines[1:])
            lines.append("```")

    return "\n".join(lines)


async def compare_format(
    ruff_baseline_executable: Path,
    ruff_comparison_executable: Path,
    options: FormatOptions,
    config_overrides: ConfigOverrides,
    cloned_repo: ClonedRepository,
    format_comparison: FormatComparison,
):
    args = (
        ruff_baseline_executable,
        ruff_comparison_executable,
        options,
        config_overrides,
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
    ruff_baseline_executable: Path,
    ruff_comparison_executable: Path,
    options: FormatOptions,
    config_overrides: ConfigOverrides,
    cloned_repo: ClonedRepository,
) -> Sequence[str]:
    with config_overrides.patch_config(cloned_repo.path, options.preview):
        # Run format to get the baseline
        await format(
            executable=ruff_baseline_executable.resolve(),
            path=cloned_repo.path,
            name=cloned_repo.fullname,
            options=options,
        )
        # Then get the diff from stdout
        diff = await format(
            executable=ruff_comparison_executable.resolve(),
            path=cloned_repo.path,
            name=cloned_repo.fullname,
            options=options,
            diff=True,
        )
    return diff


async def format_and_format(
    ruff_baseline_executable: Path,
    ruff_comparison_executable: Path,
    options: FormatOptions,
    config_overrides: ConfigOverrides,
    cloned_repo: ClonedRepository,
) -> Sequence[str]:
    with config_overrides.patch_config(cloned_repo.path, options.preview):
        # Run format without diff to get the baseline
        await format(
            executable=ruff_baseline_executable.resolve(),
            path=cloned_repo.path,
            name=cloned_repo.fullname,
            options=options,
        )

    # Commit the changes
    commit = await cloned_repo.commit(
        message=f"Formatted with baseline {ruff_baseline_executable}"
    )
    # Then reset
    await cloned_repo.reset()

    with config_overrides.patch_config(cloned_repo.path, options.preview):
        # Then run format again
        await format(
            executable=ruff_comparison_executable.resolve(),
            path=cloned_repo.path,
            name=cloned_repo.fullname,
            options=options,
        )

    # Then get the diff from the commit
    diff = await cloned_repo.diff(commit)

    return diff


async def format(
    *,
    executable: Path,
    path: Path,
    name: str,
    options: FormatOptions,
    diff: bool = False,
) -> Sequence[str]:
    """Run the given ruff binary against the specified path."""
    args = options.to_ruff_args()
    logger.debug(f"Formatting {name} with {executable} " + " ".join(args))

    if diff:
        args.append("--diff")

    start = time.time()
    proc = await create_subprocess_exec(
        executable.absolute(),
        *args,
        ".",
        stdout=PIPE,
        stderr=PIPE,
        cwd=path,
    )
    result, err = await proc.communicate()
    end = time.time()

    logger.debug(f"Finished formatting {name} with {executable} in {end - start:.2f}s")

    if proc.returncode not in [0, 1]:
        raise ToolError(err.decode("utf8"))

    lines = result.decode("utf8").splitlines()
    return lines


class FormatComparison(Enum):
    # Run baseline executable then comparison executable.
    # Checks for changes in behavior when formatting previously "formatted" code
    base_then_comp = "base-then-comp"

    # Run Ruff baseline then reset and run Ruff comparison.
    # Checks changes in behavior when formatting "unformatted" code
    base_and_comp = "base-and-comp"
