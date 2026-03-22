from __future__ import annotations

import subprocess
import tempfile
from pathlib import Path

from fuzzer import logger


def minimize(code: str, *, bug_kind: str, test_executable: Path) -> str:
    """Attempt to minimize a failing template input.

    Uses line-level binary search removal to find a smaller reproducer.
    Falls back to returning the original code if minimization fails.
    """
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
    lines: list[str], *, bug_kind: str, test_executable: Path
) -> list[str]:
    """Remove lines using binary search while the bug still reproduces."""
    current = list(lines)

    # Try removing chunks of decreasing size
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
                # Don't advance i since we removed elements at this position
            else:
                i += chunk_size

        chunk_size //= 2

    return current


def _reproduces_bug(code: str, *, bug_kind: str, test_executable: Path) -> bool:
    """Check if the given code still triggers the same bug kind."""
    with tempfile.NamedTemporaryFile(mode="w", suffix=".html", delete=False) as f:
        f.write(code)
        f.flush()
        tmp_path = Path(f.name)

    try:
        result = subprocess.run(
            [str(test_executable), "format", str(tmp_path)],
            capture_output=True,
            timeout=10,
        )

        if bug_kind == "crash":
            return result.returncode >= 2 or result.returncode < 0

        if bug_kind == "idempotency" and result.returncode == 0:
            formatted = tmp_path.read_text()

            with tempfile.NamedTemporaryFile(
                mode="w", suffix=".html", delete=False
            ) as f2:
                f2.write(formatted)
                f2.flush()
                tmp_path2 = Path(f2.name)

            try:
                result2 = subprocess.run(
                    [str(test_executable), "format", str(tmp_path2)],
                    capture_output=True,
                    timeout=10,
                )
                if result2.returncode == 0:
                    return formatted != tmp_path2.read_text()
            except subprocess.TimeoutExpired:
                return True
            finally:
                tmp_path2.unlink(missing_ok=True)

    except subprocess.TimeoutExpired:
        return bug_kind == "crash"
    finally:
        tmp_path.unlink(missing_ok=True)

    return False


def write_failure(seed: int, code: str) -> Path:
    """Write a minimized failure to a file and return the path."""
    path = Path(f"fuzz-failure-{seed}.html")
    path.write_text(code)
    logger.info("Wrote failure to %s", path)
    return path
