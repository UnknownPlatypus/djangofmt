from __future__ import annotations

from pathlib import Path

from fuzzer import logger
from fuzzer.run import BugKind, check_template


def minimize(code: str, *, bug_kind: BugKind, test_executable: Path) -> str:
    """Shrink a failing template to the smallest reproducer via line-level removal."""
    lines = code.splitlines(keepends=True)

    if len(lines) <= 1:
        return code

    minimized_lines = _minimize_lines(
        lines, bug_kind=bug_kind, test_executable=test_executable
    )
    result = "".join(minimized_lines)

    if len(result) < len(code):
        logger.info(
            "Minimized from %d to %d chars (%d to %d lines)",
            len(code),
            len(result),
            len(lines),
            len(minimized_lines),
        )

    return result


def _minimize_lines(
    lines: list[str], *, bug_kind: BugKind, test_executable: Path
) -> list[str]:
    current = list(lines)

    chunk_size = len(current) // 2
    while chunk_size >= 1:
        i = 0
        while i < len(current):
            end = min(i + chunk_size, len(current))
            candidate = current[:i] + current[end:]

            if not candidate:
                i += chunk_size
                continue

            candidate_code = "".join(candidate)
            if _reproduces_bug(
                candidate_code, bug_kind=bug_kind, test_executable=test_executable
            ):
                current = candidate
            else:
                i += chunk_size

        chunk_size //= 2

    return current


def _reproduces_bug(code: str, *, bug_kind: BugKind, test_executable: Path) -> bool:
    result = check_template(code, executable=test_executable, timeout=10)
    return result == bug_kind


def write_failure(seed: int, code: str) -> Path:
    path = Path(f"fuzz-failure-{seed}.html")
    path.write_text(code)
    return path
