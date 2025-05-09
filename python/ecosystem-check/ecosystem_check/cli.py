from __future__ import annotations

import argparse
import asyncio
import logging
import os
import shutil
import sys
import sysconfig
import tempfile
from contextlib import nullcontext
from pathlib import Path
from signal import SIGINT, SIGTERM
from types import TracebackType
from typing import Literal

from ecosystem_check import logger
from ecosystem_check.defaults import DEFAULT_TARGETS
from ecosystem_check.format import FormatComparison
from ecosystem_check.main import OutputFormat, main
from ecosystem_check.projects import Command


def excepthook(
    type: type[BaseException], value: BaseException, tb: TracebackType | None
) -> None:
    if hasattr(sys, "ps1") or not sys.stderr.isatty():
        # we are in interactive mode or we don't have a tty so call the default
        sys.__excepthook__(type, value, tb)
    else:
        import pdb
        import traceback

        traceback.print_exception(type, value, tb)
        print()
        pdb.post_mortem(tb)


def entrypoint() -> None:
    args = parse_args()

    if args.pdb:
        sys.excepthook = excepthook

    if args.verbose:
        logging.basicConfig(level=logging.DEBUG)
    else:
        logging.basicConfig(level=logging.INFO)

    baseline_executable = resolve_executable(args.baseline_executable, "baseline")
    comparison_executable = resolve_executable(args.comparison_executable, "comparison")

    # Use a temporary directory for caching if no cache is specified
    cache_context = (
        tempfile.TemporaryDirectory()
        if not args.cache_dir
        else nullcontext(args.cache_dir)
    )
    with cache_context as cache:
        loop = asyncio.get_event_loop()
        main_task = asyncio.ensure_future(
            main(
                command=Command(args.command),
                baseline_executable=baseline_executable,
                comparison_executable=comparison_executable,
                targets=DEFAULT_TARGETS,
                output_format=OutputFormat(args.output_format),
                project_dir=Path(cache),
                raise_on_failure=args.pdb,
                format_comparison=(
                    FormatComparison(args.format_comparison)
                    if args.command == Command.format
                    else None
                ),
            )
        )
        # https://stackoverflow.com/a/58840987/3549270
        for signal in [SIGINT, SIGTERM]:
            loop.add_signal_handler(signal, main_task.cancel)
        try:
            loop.run_until_complete(main_task)
        finally:
            loop.close()


def parse_args() -> argparse.Namespace:
    """Parse command line arguments."""
    parser = argparse.ArgumentParser(
        description="Check two versions of an executable against a corpus of open-source code."
    )
    parser.add_argument(
        "--cache-dir",
        type=Path,
        help="Location for caching cloned repositories",
    )
    parser.add_argument(
        "--output-format",
        choices=list(OutputFormat),
        default=OutputFormat.MARKDOWN,
        help="The format in which the output should be generated.",
    )
    parser.add_argument(
        "-v",
        "--verbose",
        action="store_true",
        help="Enable debug logging",
    )
    parser.add_argument(
        "--pdb",
        action="store_true",
        help="Enable debugging on failure",
    )
    parser.add_argument(
        "--format-comparison",
        choices=list(FormatComparison),
        default=FormatComparison.BASE_AND_COMP,
        help="Type of comparison to make when checking formatting.",
    )
    parser.add_argument(
        "command",
        choices=list(Command),
        help="The command to test",
    )
    parser.add_argument(
        "baseline_executable",
        type=Path,
    )
    parser.add_argument(
        "comparison_executable",
        type=Path,
    )
    # https://docs.python.org/3.14/library/argparse.html#suggest-on-error
    parser.suggest_on_error = True  # type: ignore[attr-defined]
    return parser.parse_args()


def _get_executable_path(name: str) -> Path | None:
    # Add suffix for Windows executables
    name += ".exe" if sys.platform == "win32" and not name.endswith(".exe") else ""

    path = os.path.join(sysconfig.get_path("scripts"), name)

    # The executable in the current interpreter's scripts directory.
    if os.path.exists(path):
        return Path(path)

    # The executable in the global environment.
    environment_path = shutil.which(name)
    if environment_path:
        return Path(environment_path)

    return None


def resolve_executable(
    executable: Path, executable_type: Literal["baseline", "comparison"]
) -> Path:
    if executable.exists():
        return executable

    resolved_executable = _get_executable_path(str(executable))
    if not resolved_executable:
        print(
            (
                f"Could not find djangofmt {executable_type} "
                f"executable: {resolved_executable}"
            ),
            sys.stderr,
        )
        exit(1)
    logger.info(
        "Resolved %s executable %s to %s",
        executable_type,
        executable,
        resolved_executable,
    )
    return resolved_executable
