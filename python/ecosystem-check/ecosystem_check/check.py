"""
Execution, comparison, and summary of `djangofmt check` ecosystem checks.
"""

from __future__ import annotations

import glob
import time
from asyncio import create_subprocess_exec
from pathlib import Path
from subprocess import PIPE
from typing import TYPE_CHECKING

from ecosystem_check import logger
from ecosystem_check.format import add_s
from ecosystem_check.markdown import markdown_project_section
from ecosystem_check.projects import Command
from ecosystem_check.types import (
    CheckDiff,
    Comparison,
    Diff,
    Result,
    ToolError,
)

if TYPE_CHECKING:
    from ecosystem_check.projects import (
        CliOptions,
        ClonedRepository,
    )


def markdown_check_result(result: Result) -> str:
    """
    Render a `djangofmt check` ecosystem check result as markdown.
    """
    error_count = len(result.errored)
    diffs = [
        comp.diff for _, comp in result.completed if isinstance(comp.diff, CheckDiff)
    ]
    projects_with_changes = sum(bool(d) for d in diffs)
    total_added = sum(d.diagnostics_added for d in diffs)
    total_removed = sum(d.diagnostics_removed for d in diffs)
    total_fixed_added = sum(d.fix_diff.lines_added for d in diffs)
    total_fixed_removed = sum(d.fix_diff.lines_removed for d in diffs)

    if projects_with_changes == 0 and error_count == 0:
        return "\u2705 ecosystem check detected no check changes."

    lines: list[str] = []
    if projects_with_changes == 0:
        lines.append(
            f"\u2139\ufe0f ecosystem check **encountered check errors**. "
            f"(no diagnostic changes; {error_count} project error{add_s(error_count)})"
        )
    else:
        changes = (
            f"+{total_added} -{total_removed} diagnostic lines, "
            f"+{total_fixed_added} -{total_fixed_removed} fixed lines "
            f"in {projects_with_changes} project{add_s(projects_with_changes)}"
        )

        if error_count:
            changes += f"; {error_count} project error{add_s(error_count)}"

        unchanged_projects = len(result.completed) - projects_with_changes
        if unchanged_projects:
            changes += (
                f"; {unchanged_projects} project{add_s(unchanged_projects)} unchanged"
            )

        lines.append(
            f"\u2139\ufe0f ecosystem check **detected check changes**. ({changes})"
        )

    lines.append("")

    for project, comparison in result.completed:
        diff = comparison.diff
        if not diff or not isinstance(diff, CheckDiff):
            continue

        title = (
            f"+{diff.diagnostics_added} -{diff.diagnostics_removed} diagnostic lines"
        )
        if diff.fix_diff:
            files = diff.fix_diff.modified_files
            title += (
                f", +{diff.fix_diff.lines_added} -{diff.fix_diff.lines_removed} "
                f"fixed lines across {files} file{add_s(files)}"
            )

        lines.extend(
            markdown_project_section(
                title=title,
                content=diff.format_markdown(repo=comparison.repo),
                options=project.cli_options,
                project=project,
            )
        )

    for project, error in result.errored:
        lines.extend(
            markdown_project_section(
                title="error",
                content=f"```\n{str(error).strip()}\n```",
                options=project.cli_options,
                project=project,
            )
        )

    return "\n".join(lines)


async def compare_check(
    baseline_executable: Path,
    comparison_executable: Path,
    options: CliOptions,
    cloned_repo: ClonedRepository,
) -> Comparison:
    # Both runs apply their fixes, so fixable rules are compared as a source diff and only the unfixable diagnostics are compared as text.
    baseline_output = await check(
        executable=baseline_executable.resolve(),
        path=cloned_repo.path,
        repo_fullname=cloned_repo.fullname,
        options=options,
    )
    commit = await cloned_repo.commit(
        message=f"Fixed with baseline {baseline_executable}"
    )
    await cloned_repo.reset(cloned_repo.path)

    comparison_output = await check(
        executable=comparison_executable.resolve(),
        path=cloned_repo.path,
        repo_fullname=cloned_repo.fullname,
        options=options,
    )

    return Comparison(
        diff=CheckDiff(
            baseline_output,
            comparison_output,
            Diff(await cloned_repo.diff(commit)),
        ),
        repo=cloned_repo,
    )


async def check(
    *,
    executable: Path,
    path: Path,
    repo_fullname: str,
    options: CliOptions,
) -> str:
    """Run `djangofmt check --fix` against the path, returning unfixed diagnostics."""
    # The check CLI does not support --custom-blocks
    args = ["check", *options.to_args(executable.name, command=Command.CHECK)]
    files = set(
        glob.iglob("**/*templates/**/*.html", recursive=True, root_dir=path)
    ) - set(options.excluded_files())

    if not files:
        logger.debug(f"No files to check for {repo_fullname}")
        return ""

    logger.debug(
        f"Checking {repo_fullname} with cmd {executable!r} ({len(files)} files)"
    )
    if options.exclude:
        logger.debug(f"Excluding {options.exclude}")

    start = time.perf_counter()
    proc = await create_subprocess_exec(
        executable.absolute(),
        *args,
        *sorted(files),
        stdout=PIPE,
        stderr=PIPE,
        cwd=path,
    )
    _, err = await proc.communicate()
    end = time.perf_counter()

    logger.debug(
        f"Finished checking {repo_fullname} with {executable} in {end - start:.2f}s"
    )

    if proc.returncode not in [0, 1]:
        raise ToolError(err.decode("utf8"))

    # Diagnostics are reported to stderr, minus the per-run summary line we want to drop here
    return "".join(
        line
        for line in err.decode("utf8").splitlines(keepends=True)
        if not line.startswith(("Found ", "All checks passed!"))
    )
