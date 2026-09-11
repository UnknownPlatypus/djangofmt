"""
Execution and summary of the parse check: formatting must keep templates parseable.
"""

from __future__ import annotations

import glob
from pathlib import Path
from typing import TYPE_CHECKING

from ecosystem_check.format import add_s, format
from ecosystem_check.markdown import markdown_project_section
from ecosystem_check.oracle import parse_django, parse_jinja
from ecosystem_check.projects import Profile
from ecosystem_check.types import Comparison, ParseRegressions, Result

if TYPE_CHECKING:
    from ecosystem_check.projects import CliOptions, ClonedRepository


def markdown_validate_result(result: Result, title: str) -> str:
    """
    Render a parse check ecosystem result as markdown.
    """
    regressions = [
        (project, comparison.repo, comparison.diff)
        for project, comparison in result.completed
        if isinstance(comparison.diff, ParseRegressions) and comparison.diff
    ]
    if not regressions and not result.errored:
        return f"✅ {title}"

    templates = sum(len(diff) for _, _, diff in regressions)
    summary = (
        f"{templates} parse regression{add_s(templates)} "
        f"in {len(regressions)} project{add_s(len(regressions))}"
        if regressions
        else "no parse regressions"
    )
    if result.errored:
        summary += f"; {len(result.errored)} project error{add_s(len(result.errored))}"
    status = "❌" if regressions else "\u2139\ufe0f"
    lines = [f"{status} {title}: {summary}", ""]

    for project, repo, diff in regressions:
        lines.extend(
            markdown_project_section(
                title=f"{len(diff)} parse regression{add_s(len(diff))}",
                content=diff.format_markdown(repo),
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


async def validate_project(
    executable: Path, options: CliOptions, cloned_repo: ClonedRepository
) -> Comparison:
    """Format the checkout in place and report the templates that no longer parse."""
    parse = parse_jinja if options.profile is Profile.JINJA else parse_django
    files = set(
        glob.iglob("**/*templates/**/*.html", recursive=True, root_dir=cloned_repo.path)
    ) - set(options.excluded_files())

    def read(file: str) -> str:
        return cloned_repo.path.joinpath(file).read_text("utf8", errors="replace")

    # A template that is already invalid cannot tell us anything about the formatter.
    parseable = sorted(file for file in files if parse(read(file), file) is None)
    await format(
        executable=executable.resolve(),
        path=cloned_repo.path,
        repo_fullname=cloned_repo.fullname,
        options=options,
    )

    regressions = ParseRegressions()
    for file in parseable:
        if error := parse(read(file), file):
            regressions[file] = error
    return Comparison(diff=regressions, repo=cloned_repo)
