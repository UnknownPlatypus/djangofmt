from __future__ import annotations

from concurrent.futures import ProcessPoolExecutor
from dataclasses import dataclass
from functools import partial
from pathlib import Path

from fuzzer import logger
from fuzzer.generate import TemplateGenerator
from fuzzer.minimize import minimize
from fuzzer.run import BugKind, check_template


@dataclass
class FuzzResult:
    seed: int
    bug_kind: BugKind
    code: str


def fuzz_seed(
    seed: int,
    *,
    test_executable: Path,
    baseline_executable: Path | None = None,
    only_new_bugs: bool = False,
    profile: str = "django",
) -> FuzzResult | None:
    generator = TemplateGenerator(seed, profile=profile)
    code = generator.generate()

    bug = check_template(code, executable=test_executable)
    if bug is None:
        return None

    if only_new_bugs and baseline_executable is not None:
        baseline_bug = check_template(code, executable=baseline_executable)
        if baseline_bug is not None:
            logger.debug("Seed %d: bug also in baseline, skipping", seed)
            return None

    minimized = minimize(code, bug_kind=bug, test_executable=test_executable)
    return FuzzResult(seed=seed, bug_kind=bug, code=minimized)


def run_fuzz(
    seeds: list[int],
    *,
    test_executable: Path,
    baseline_executable: Path | None = None,
    only_new_bugs: bool = False,
    profile: str = "django",
    workers: int = 1,
) -> list[FuzzResult]:
    fuzz = partial(
        fuzz_seed,
        test_executable=test_executable,
        baseline_executable=baseline_executable,
        only_new_bugs=only_new_bugs,
        profile=profile,
    )

    bugs: list[FuzzResult] = []
    if workers <= 1 or len(seeds) <= 10:
        for seed in seeds:
            logger.debug("Fuzzing seed %d", seed)
            result = fuzz(seed)
            if result is not None:
                bugs.append(result)
                logger.info("Bug found at seed %d: %s", result.seed, result.bug_kind)
    else:
        with ProcessPoolExecutor(max_workers=workers) as executor:
            for result in executor.map(fuzz, seeds):
                if result is not None:
                    bugs.append(result)
                    logger.info(
                        "Bug found at seed %d: %s", result.seed, result.bug_kind
                    )

    return bugs
