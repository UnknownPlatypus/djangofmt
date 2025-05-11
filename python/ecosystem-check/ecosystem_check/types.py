from __future__ import annotations

import dataclasses
from collections.abc import Iterable, Iterator
from dataclasses import dataclass, is_dataclass
from functools import cached_property
from typing import TYPE_CHECKING, Any

from unidiff import PatchSet

if TYPE_CHECKING:
    from ecosystem_check.projects import ClonedRepository, Project


class Serializable:
    """
    Allows serialization of content by casting to a JSON-compatible type.
    """

    def jsonable(self) -> Any:
        # Default implementation for dataclasses
        if is_dataclass(self) and not isinstance(self, type):
            return dataclasses.asdict(self)

        raise NotImplementedError()


class Diff(Serializable):
    """A full Diff object for a project or a Hunk"""

    def __init__(self, lines: Iterable[str], leading_spaces: int = 0) -> None:
        self.lines = list(lines)

        # Compute added and removed lines once
        self.added = [
            line[2:]
            for line in self.lines
            if line.startswith("+" + " " * leading_spaces)
            # Do not include patch headers
            and not line.startswith("+++")
        ]
        self.removed = [
            line[2:]
            for line in self.lines
            if line.startswith("-" + " " * leading_spaces)
            # Do not include patch headers
            and not line.startswith("---")
        ]

    def __bool__(self) -> bool:
        return bool(self.added or self.removed)

    def __iter__(self) -> Iterator[str]:
        yield from self.lines

    @cached_property
    def patch_set(self) -> PatchSet:
        return PatchSet("\n".join(self.lines))

    @property
    def lines_added(self) -> int:
        return len(self.added)

    @property
    def lines_removed(self) -> int:
        return len(self.removed)

    def jsonable(self) -> Any:
        return self.lines


class Diffs(list[Diff]):
    """A collection of Diff objects with a few added properties"""

    @property
    def all_empty(self) -> bool:
        return all(not diff for diff in self)

    @property
    def modified_files(self) -> int:
        file_paths = set()
        for diff in self:
            for patch_file in diff.patch_set.modified_files:
                file_paths.add(patch_file.path)

        return len(file_paths)

    @property
    def lines_added(self) -> int:
        return sum(diff.patch_set.added for diff in self)

    @property
    def lines_removed(self) -> int:
        return sum(diff.patch_set.removed for diff in self)


@dataclass(frozen=True)
class HunkDetail:
    """The minimal details of a patch hunk that makes it unique."""

    path: str
    start: int
    length: int


@dataclass(frozen=True)
class Result(Serializable):
    """
    The result of an ecosystem check for a collection of projects.
    """

    errored: list[tuple[Project, BaseException]]
    completed: list[tuple[Project, Comparison]]


@dataclass(frozen=True)
class Comparison(Serializable):
    """
    The result of a completed ecosystem comparison for a single project.
    """

    diffs: Diffs
    repo: ClonedRepository


class ToolError(Exception):
    """An error reported by the checked executable."""
