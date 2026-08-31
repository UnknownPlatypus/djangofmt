"""
Abstractions and utilities for working with projects to run ecosystem checks on.
"""

from __future__ import annotations

import enum
from asyncio import create_subprocess_exec
from dataclasses import dataclass, field
from pathlib import Path
from subprocess import DEVNULL, PIPE
from typing import Self

from ecosystem_check import logger
from ecosystem_check.types import HunkDetail, Serializable


@dataclass(frozen=True, slots=True)
class Project(Serializable):
    """
    An ecosystem target
    """

    repo: Repository
    cli_options: CliOptions = field(default_factory=lambda: CliOptions())


class Command(enum.StrEnum):
    FORMAT = enum.auto()
    CHECK = enum.auto()


class GitDomain(enum.StrEnum):
    GITHUB = "github.com"
    CODEBERG = "codeberg.org"

    @property
    def blob_path(self) -> str:
        return {
            GitDomain.GITHUB: "blob",
            GitDomain.CODEBERG: "src/commit",
        }[self]


class Formatter(enum.StrEnum):
    """A tool name expected to do formatting work on files"""

    DJANGOFMT = "djangofmt"
    DJADE = "djade"
    RUSTYWIND = "rustywind"


class Profile(enum.StrEnum):
    """A tool name expected to do formatting work on files"""

    DJANGO = "django"
    JINJA = "jinja"


class ExcludeReason(enum.StrEnum):
    """Why a template is excluded from the ecosystem check."""

    # An element's open and close tags sit in different template branches or blocks,
    # so no tree can be built. https://github.com/g-plane/markup_fmt/issues/97
    TAG_SPANS_TEMPLATE_BLOCK = enum.auto()

    # A templated element name, e.g. `<{% if href %}a{% else %}button{% endif %}>`.
    DYNAMIC_TAG_NAME = enum.auto()

    # A template tag standing in for an attribute name, e.g. `{% formaction %}="{{ url }}"`.
    TEMPLATE_TAG_AS_ATTRIBUTE = enum.auto()

    # An end tag HTML5 allows omitting (`</li>`, `</p>`, `</tr>`, `</body>`, ...).
    # Legal, but discouraged enough that we keep requiring it.
    MISSING_END_TAG = enum.auto()

    # Malformed markup in the upstream project.
    INVALID_SOURCE_HTML = enum.auto()

    # Upstream fixture that is deliberately unparsable.
    INTENTIONALLY_INVALID = enum.auto()

    # A Jinja/Sphinx template living in a project checked under the Django profile,
    # using syntax Django has no equivalent for (`{%- -%}` whitespace control).
    FOREIGN_TEMPLATE_ENGINE = enum.auto()

    # Parses fine but formatting never converges.
    UNSTABLE_FORMATTING = enum.auto()

    # Root cause not identified yet.
    UNKNOWN = enum.auto()


@dataclass(frozen=True, slots=True)
class CliOptions(Serializable):
    """
    Option to pass to the cli.
    """

    profile: Profile = Profile.DJANGO
    custom_blocks: str = ""  # Comma-separated list of custom blocks
    exclude: dict[ExcludeReason, tuple[str, ...]] = field(default_factory=dict)

    def to_args(self, executable_name: str, *, command: Command) -> list[str]:
        if Formatter.DJANGOFMT in executable_name:
            args = ["--profile", self.profile]
            if command is Command.FORMAT and self.custom_blocks:
                args.extend(("--custom-blocks", self.custom_blocks))
            if command is Command.CHECK:
                # Select every rule, including preview, for maximum ecosystem coverage.
                # Fixes are applied so they can be reviewed as a source diff.
                args.extend(
                    (
                        "--select",
                        "category:all",
                        "--preview",
                        "--fix",
                        "--unsafe-fixes",
                        "--output-format",
                        "concise",
                    )
                )
            return args
        elif executable_name == Formatter.DJADE:
            return []
        elif executable_name == Formatter.RUSTYWIND:
            return ["--write"]
        raise AssertionError(
            f"Cannot cast format options for this executable: {executable_name}"
        )

    def excluded_files(self) -> tuple[str, ...]:
        return tuple(file for files in self.exclude.values() for file in files)

    def reason_for(self, file: str) -> ExcludeReason:
        return next(reason for reason, files in self.exclude.items() if file in files)


class ProjectSetupError(Exception):
    """An error setting up a project."""


@dataclass(frozen=True, slots=True)
class Repository(Serializable):
    """
    A remote GitHub repository.
    """

    owner: str
    name: str
    ref: str | None
    domain: GitDomain = GitDomain.GITHUB

    @property
    def fullname(self) -> str:
        return f"{self.owner}/{self.name}"

    @property
    def url(self: Self) -> str:
        return f"https://{self.domain}/{self.fullname}"

    async def clone(self: Self, checkout_dir: Path) -> ClonedRepository:
        """
        Shallow clone this repository
        """
        if checkout_dir.exists():
            logger.debug(f"Reusing cached {self.fullname}")

            if self.ref:
                logger.debug(f"Checking out {self.fullname} @ {self.ref}")

                process = await create_subprocess_exec(
                    *["git", "checkout", "-f", self.ref],
                    cwd=checkout_dir,
                    env={"GIT_TERMINAL_PROMPT": "0"},
                    stdout=PIPE,
                    stderr=PIPE,
                )
                if await process.wait() != 0:
                    _, stderr = await process.communicate()
                    raise ProjectSetupError(
                        f"Failed to checkout {self.ref}: {stderr.decode()}"
                    )

            await self.reset(checkout_dir)
            cloned_repo = await ClonedRepository.from_path(checkout_dir, self)

            logger.debug(f"Pulling latest changes for {self.fullname} @ {self.ref}")
            await cloned_repo.pull()

            return cloned_repo

        logger.debug(f"Cloning {self.owner}:{self.name} to {checkout_dir}")
        command = [
            "git",
            "clone",
            "--config",
            "advice.detachedHead=false",
            "--quiet",
            "--depth",
            "1",
            "--no-tags",
        ]
        if self.ref:
            command.extend(["--branch", self.ref])

        command.extend(
            [
                self.url,
                str(checkout_dir),
            ],
        )

        process = await create_subprocess_exec(
            *command,
            env={"GIT_TERMINAL_PROMPT": "0"},
            stdout=PIPE,
            stderr=PIPE,
        )

        if await process.wait() != 0:
            _, stderr = await process.communicate()
            raise ProjectSetupError(
                f"Failed to clone {self.fullname}: {stderr.decode()}"
            )

        # Configure git user — needed for `self.commit` to work
        await (
            await create_subprocess_exec(
                *["git", "config", "user.email", "thibaut.decombe+bot@gmail.com"],
                cwd=checkout_dir,
                env={"GIT_TERMINAL_PROMPT": "0"},
                stdout=DEVNULL,
                stderr=DEVNULL,
            )
        ).wait()

        await (
            await create_subprocess_exec(
                *["git", "config", "user.name", "Ecosystem Bot"],
                cwd=checkout_dir,
                env={"GIT_TERMINAL_PROMPT": "0"},
                stdout=DEVNULL,
                stderr=DEVNULL,
            )
        ).wait()

        return await ClonedRepository.from_path(checkout_dir, self)

    async def reset(self: Self, checkout_dir: Path) -> None:
        """
        Reset the cloned repository to the ref it started at.
        """
        process = await create_subprocess_exec(
            *["git", "reset", "--hard", *(["origin/" + self.ref] if self.ref else [])],
            cwd=checkout_dir,
            env={"GIT_TERMINAL_PROMPT": "0"},
            stdout=PIPE,
            stderr=PIPE,
        )
        _, stderr = await process.communicate()
        if await process.wait() != 0:
            raise RuntimeError(f"Failed to reset: {stderr.decode()}")


@dataclass(frozen=True, slots=True)
class ClonedRepository(Repository, Serializable):
    """
    A cloned GitHub repository, which includes the hash of the current commit.
    """

    commit_hash: str
    path: Path
    domain: GitDomain

    @classmethod
    async def from_path(cls, path: Path, repo: Repository) -> Self:
        return cls(
            name=repo.name,
            owner=repo.owner,
            ref=repo.ref,
            domain=repo.domain,
            path=path,
            commit_hash=await cls._get_head_commit(path),
        )

    def url_for(
        self: Self,
        path: str,
        line_number: int | None = None,
        end_line_number: int | None = None,
    ) -> str:
        """
        Return the remote GitHub URL for the given path in this repository.
        """
        url = f"{self.url}/{self.domain.blob_path}/{self.commit_hash}/{path}"
        if line_number:
            url += f"#L{line_number}"
        if end_line_number:
            url += f"-L{end_line_number}"
        return url

    @staticmethod
    async def _get_head_commit(checkout_dir: Path) -> str:
        """
        Return the commit sha for the repository in the checkout directory.
        """
        process = await create_subprocess_exec(
            *["git", "rev-parse", "HEAD"],
            cwd=checkout_dir,
            stdout=PIPE,
        )
        stdout, _ = await process.communicate()
        if await process.wait() != 0:
            raise ProjectSetupError(f"Failed to retrieve commit sha at {checkout_dir}")

        return stdout.decode().strip()

    async def pull(self: Self) -> None:
        """
        Pull the latest changes.

        Typically, `reset` should be run first.
        """
        process = await create_subprocess_exec(
            *["git", "pull"],
            cwd=self.path,
            env={"GIT_TERMINAL_PROMPT": "0"},
            stdout=PIPE,
            stderr=PIPE,
        )
        _, stderr = await process.communicate()
        if await process.wait() != 0:
            raise RuntimeError(f"Failed to pull: {stderr.decode()}")

    async def commit(self: Self, message: str) -> str:
        """
        Commit all current changes.

        Empty commits are allowed.
        """
        process = await create_subprocess_exec(
            *["git", "commit", "--allow-empty", "-a", "-m", message],
            cwd=self.path,
            env={"GIT_TERMINAL_PROMPT": "0"},
            stdout=PIPE,
            stderr=PIPE,
        )
        _, stderr = await process.communicate()
        if await process.wait() != 0:
            raise RuntimeError(f"Failed to commit: {stderr.decode()}")

        return await self._get_head_commit(self.path)

    async def diff(self: Self, *args: str) -> list[str]:
        """
        Get the current diff from git.

        Arguments are passed to `git diff ...`
        """
        process = await create_subprocess_exec(
            *["git", "diff", *args],
            cwd=self.path,
            env={"GIT_TERMINAL_PROMPT": "0"},
            stdout=PIPE,
            stderr=PIPE,
        )
        stdout, stderr = await process.communicate()
        if await process.wait() != 0:
            raise RuntimeError(f"Failed to commit: {stderr.decode()}")

        return stdout.decode().splitlines()

    async def diff_for_hunk(
        self: Self, hunk_range: HunkDetail, *args: str
    ) -> list[str]:
        """
        Get the current diff for a specific file and range.

        Arguments are passed to `git log ...`
        """
        process = await create_subprocess_exec(
            *[
                "git",
                "log",
                "--reverse",
                "-L",
                f"{hunk_range.start},+{hunk_range.length}:./{hunk_range.path}",
                *args,
            ],
            cwd=self.path,
            env={"GIT_TERMINAL_PROMPT": "0"},
            stdout=PIPE,
            stderr=PIPE,
        )
        stdout, stderr = await process.communicate()
        if await process.wait() != 0:
            raise RuntimeError(f"Failed to commit: {stderr.decode()}")

        return stdout.decode().splitlines()
