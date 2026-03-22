from __future__ import annotations

import subprocess
import tempfile
from concurrent.futures import ProcessPoolExecutor
from dataclasses import dataclass
from pathlib import Path

from fuzzer import logger
from fuzzer.generate import TemplateGenerator
from fuzzer.minimize import minimize


@dataclass
class FuzzResult:
    seed: int
    bug_kind: str | None  # None, "crash", "idempotency"
    code: str


def fuzz_seed(
    seed: int,
    *,
    test_executable: Path,
    baseline_executable: Path | None = None,
    only_new_bugs: bool = False,
    profile: str = "django",
) -> FuzzResult | None:
    """Run the fuzzer for a single seed. Returns a FuzzResult if a bug is found."""
    generator = TemplateGenerator(seed, profile=profile)
    code = generator.generate()

    bug = _check_single(code, test_executable=test_executable)

    if bug is None:
        return None

    # If differential mode, check baseline too
    if only_new_bugs and baseline_executable is not None:
        baseline_bug = _check_single(code, test_executable=baseline_executable)
        if baseline_bug is not None:
            # Bug exists in baseline too, skip
            logger.debug("Seed %d: bug also in baseline, skipping", seed)
            return None

    # Try to minimize
    minimized = minimize(code, bug_kind=bug, test_executable=test_executable)

    return FuzzResult(seed=seed, bug_kind=bug, code=minimized)


def _check_single(code: str, *, test_executable: Path) -> str | None:
    """Check a single template. Returns bug kind or None."""
    with tempfile.NamedTemporaryFile(mode="w", suffix=".html", delete=False) as f:
        f.write(code)
        f.flush()
        tmp_path = Path(f.name)

    try:
        result = subprocess.run(
            [str(test_executable), "format", str(tmp_path)],
            capture_output=True,
            timeout=30,
        )

        # Exit code >= 2 means a crash/error (not a parse error)
        if result.returncode >= 2:
            logger.debug("Crash detected (exit code %d)", result.returncode)
            return "crash"

        # Killed by signal (negative return code on Unix)
        if result.returncode < 0:
            logger.debug("Killed by signal %d", -result.returncode)
            return "crash"

        # If formatting succeeded, check idempotency
        if result.returncode == 0:
            formatted = tmp_path.read_text()

            # Write formatted output and format again
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
                    timeout=30,
                )

                if result2.returncode == 0:
                    reformatted = tmp_path2.read_text()
                    if formatted != reformatted:
                        logger.debug("Idempotency bug detected")
                        return "idempotency"
            except subprocess.TimeoutExpired:
                logger.debug("Timeout on second format pass")
                return "crash"
            finally:
                tmp_path2.unlink(missing_ok=True)

    except subprocess.TimeoutExpired:
        logger.debug("Timeout on first format pass")
        return "crash"
    finally:
        tmp_path.unlink(missing_ok=True)

    return None


def run_fuzz(
    seeds: list[int],
    *,
    test_executable: Path,
    baseline_executable: Path | None = None,
    only_new_bugs: bool = False,
    profile: str = "django",
    workers: int = 1,
) -> list[FuzzResult]:
    """Run the fuzzer across multiple seeds, returning all bugs found."""
    bugs: list[FuzzResult] = []

    if workers <= 1 or len(seeds) <= 10:
        # Sequential mode
        for seed in seeds:
            logger.debug("Fuzzing seed %d", seed)
            result = fuzz_seed(
                seed,
                test_executable=test_executable,
                baseline_executable=baseline_executable,
                only_new_bugs=only_new_bugs,
                profile=profile,
            )
            if result is not None:
                bugs.append(result)
                logger.info("Bug found at seed %d: %s", result.seed, result.bug_kind)
    else:
        # Parallel mode
        with ProcessPoolExecutor(max_workers=workers) as executor:
            futures = {
                executor.submit(
                    fuzz_seed,
                    seed,
                    test_executable=test_executable,
                    baseline_executable=baseline_executable,
                    only_new_bugs=only_new_bugs,
                    profile=profile,
                ): seed
                for seed in seeds
            }
            for future in futures:
                result = future.result()
                if result is not None:
                    bugs.append(result)
                    logger.info(
                        "Bug found at seed %d: %s", result.seed, result.bug_kind
                    )

    return bugs
