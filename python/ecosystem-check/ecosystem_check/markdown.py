from __future__ import annotations

from typing import TYPE_CHECKING

from unidiff import PatchSet

if TYPE_CHECKING:
    from ecosystem_check.projects import ClonedRepository, FormatOptions, Project


def format_patchset(
    patch_set: PatchSet, repo: ClonedRepository, commit_msgs: list[str] | None = None
) -> str:
    """
    Convert a patchset to markdown, adding permalinks to the start of each hunk.
    """
    lines: list[str] = []
    for file_patch, commit_msg in zip(
        patch_set, commit_msgs or [""] * len(patch_set), strict=False
    ):
        for hunk in file_patch:
            # Note:  When used for `format` checks, the line number is not exact because
            #        we formatted the repository for a baseline; we can't know the exact
            #        line number in the original
            #        source file.
            hunk_link = repo.url_for(file_patch.path, hunk.source_start)
            hunk_lines = str(hunk).splitlines()

            # Add a link before the hunk
            link_title = file_patch.path + "~L" + str(hunk.source_start)
            title = f"<a href='{hunk_link}'>{link_title}</a>"
            if commit_msg:
                title += f" <span style='white-space: nowrap;'>(`{commit_msg}`)</span>"
            lines.append(title)

            # Wrap the contents of the hunk in a diff code block
            lines.append("```diff")
            min_offset = min(
                len(line[1:]) - len(line[1:].lstrip(" "))
                for line in hunk_lines[1:]
                if line.strip()
            )
            lines.extend(line[:1] + line[min_offset + 1 :] for line in hunk_lines[1:])
            lines.append("```")

    return "\n".join(lines)


def markdown_project_section(
    title: str, content: str | list[str], options: FormatOptions, project: Project
) -> list[str]:
    return markdown_details(
        summary=f'<a href="{project.repo.url}">{project.repo.fullname}</a> ({title})',
        content=content,
        preface=(
            # Show the command used for the check if the options are non-default
            f"<pre>djangofmt (excluding {' '.join(options.exclude)})</pre>"
            if options.exclude
            else None
        ),
    )


def markdown_details(
    summary: str, content: str | list[str], preface: str | None
) -> list[str]:
    lines: list[str] = [f"<details><summary>{summary}</summary>"]
    if preface:
        lines.extend(("<p>", preface, "</p>"))
    lines.extend(("<p>", ""))

    if isinstance(content, str):
        lines.append(content)
    else:
        lines.extend(content)

    lines.extend(("", "</p>", "</details>"))
    return lines
