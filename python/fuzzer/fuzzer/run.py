from __future__ import annotations

import subprocess
import tempfile
from enum import StrEnum
from pathlib import Path

from fuzzer import logger


class BugKind(StrEnum):
    CRASH = "crash"
    IDEMPOTENCY = "idempotency"


def check_template(code: str, *, executable: Path, timeout: int = 30) -> BugKind | None:
    """Run djangofmt on a template and classify the result.

    Returns the kind of bug found, or None if no bug.
    Exit code 0 = success, 1 = parse error (expected), >= 2 or signal = crash.
    """
    with tempfile.NamedTemporaryFile(mode="w", suffix=".html", delete=False) as f:
        f.write(code)
        f.flush()
        tmp_path = Path(f.name)

    try:
        result = subprocess.run(
            [str(executable), "format", str(tmp_path)],
            stdout=subprocess.DEVNULL,
            stderr=subprocess.DEVNULL,
            timeout=timeout,
        )

        if result.returncode >= 2 or result.returncode < 0:
            logger.debug("Crash detected (exit code %d)", result.returncode)
            return BugKind.CRASH

        if result.returncode == 0:
            formatted = tmp_path.read_text()
            tmp_path.write_text(formatted)

            try:
                result2 = subprocess.run(
                    [str(executable), "format", str(tmp_path)],
                    stdout=subprocess.DEVNULL,
                    stderr=subprocess.DEVNULL,
                    timeout=timeout,
                )
                if result2.returncode == 0 and formatted != tmp_path.read_text():
                    logger.debug("Idempotency bug detected")
                    return BugKind.IDEMPOTENCY
            except subprocess.TimeoutExpired:
                logger.debug("Timeout on second format pass")
                return BugKind.CRASH

    except subprocess.TimeoutExpired:
        logger.debug("Timeout on first format pass")
        return BugKind.CRASH
    finally:
        tmp_path.unlink(missing_ok=True)

    return None
