from __future__ import annotations

import argparse
import logging
import os
import shutil
import sys
from pathlib import Path

from fuzzer import logger
from fuzzer.fuzz import run_fuzz
from fuzzer.minimize import write_failure


def entrypoint() -> None:
    args = parse_args()

    if args.verbose:
        logging.basicConfig(level=logging.DEBUG)
    else:
        logging.basicConfig(level=logging.INFO)

    test_executable = resolve_executable(args.test_executable)
    baseline_executable = (
        resolve_executable(args.baseline_executable)
        if args.baseline_executable
        else None
    )

    seeds = parse_seeds(args.seeds)
    logger.info(
        "Fuzzing %d seeds with %d workers (profile: %s)",
        len(seeds),
        args.workers,
        args.profile,
    )

    bugs = run_fuzz(
        seeds,
        test_executable=test_executable,
        baseline_executable=baseline_executable,
        only_new_bugs=args.only_new_bugs,
        profile=args.profile,
        workers=args.workers,
    )

    if bugs:
        logger.info("Found %d bug(s):", len(bugs))
        for bug in bugs:
            path = write_failure(bug.seed, bug.code)
            logger.info("  Seed %d: %s -> %s", bug.seed, bug.bug_kind, path)
        sys.exit(1)
    else:
        logger.info("No bugs found.")
        sys.exit(0)


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(
        description="Fuzz test djangofmt with randomly generated Django templates."
    )
    parser.add_argument(
        "seeds",
        nargs="+",
        help="Seed values or ranges (e.g., 42 0-1000 5000-6000)",
    )
    parser.add_argument(
        "--test-executable",
        type=str,
        required=True,
        help="Path to the djangofmt binary to test",
    )
    parser.add_argument(
        "--baseline-executable",
        type=str,
        default=None,
        help="Path to baseline djangofmt binary for differential testing",
    )
    parser.add_argument(
        "--only-new-bugs",
        action="store_true",
        help="Only report bugs present in test but not in baseline (requires --baseline-executable)",
    )
    parser.add_argument(
        "--profile",
        choices=["django", "jinja"],
        default="django",
        help="Template profile to generate (default: django)",
    )
    parser.add_argument(
        "--workers",
        type=int,
        default=os.cpu_count() or 1,
        help="Number of parallel workers (default: CPU count)",
    )
    parser.add_argument(
        "-v",
        "--verbose",
        action="store_true",
        help="Enable debug logging",
    )
    return parser.parse_args()


def parse_seeds(seed_args: list[str]) -> list[int]:
    """Parse seed arguments into a list of integers.

    Supports individual seeds (42) and ranges (0-1000).
    """
    seeds: list[int] = []
    for arg in seed_args:
        if "-" in arg and not arg.startswith("-"):
            parts = arg.split("-", 1)
            start = int(parts[0])
            end = int(parts[1])
            seeds.extend(range(start, end + 1))
        else:
            seeds.append(int(arg))
    return seeds


def resolve_executable(executable: str) -> Path:
    path = Path(executable)
    if path.exists():
        return path.resolve()

    # Try to find it on PATH
    resolved = shutil.which(executable)
    if resolved:
        logger.info("Resolved executable %s to %s", executable, resolved)
        return Path(resolved)

    logger.error("Could not find executable: %s", executable)
    sys.exit(1)
