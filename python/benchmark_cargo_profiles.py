# /// script
# requires-python = ">=3.11"
# dependencies = [
#     "rich",
# ]
# ///
"""
Benchmarking tool for Rust compilation profiles.

Builds binaries with various LTO and codegen-unit configurations.
Then measures binary sizes, build times, and performance using hyperfine.

It should be provided with a file containing a list of html file to format.

Usage:
    python benchmark_cargo_profiles.py --files-list /path/to/your/files-list

Output:
┏━━━━━━━━━━━┳━━━━━━━━━━━━━━━━┳━━━━━━━━━━━━━━━━━━┳━━━━━━━━━━━━━━━━━┳━━━━━━━━━━━━━━━━━━━━━━━━━┳━━━━━━━━━━━━━━━━━━━━━━━━┓
┃ Profile   ┃ Build Time (s) ┃ Binary Size (MB) ┃ Wheel Size (MB) ┃ Benchmark Time Avg (ms) ┃ Benchmark Time ±σ (ms) ┃
┡━━━━━━━━━━━╇━━━━━━━━━━━━━━━━╇━━━━━━━━━━━━━━━━━━╇━━━━━━━━━━━━━━━━━╇━━━━━━━━━━━━━━━━━━━━━━━━━╇━━━━━━━━━━━━━━━━━━━━━━━━┩
│ fatcg16   │          19.28 │             3.49 │            1.30 │                   19.26 │                   0.62 │
│ fatcg1    │          18.95 │             3.29 │            1.24 │                   19.37 │                   0.71 │
│ thincg1   │          10.87 │             3.58 │            1.27 │                   19.38 │                   0.68 │
│ thincg16  │           8.57 │             4.25 │            1.37 │                   19.40 │                   0.79 │
│ noltocg16 │           8.42 │             4.25 │            1.36 │                   19.54 │                   0.65 │
│ noltocg1  │           9.94 │             3.55 │            1.25 │                   19.67 │                   0.74 │
└───────────┴────────────────┴──────────────────┴─────────────────┴─────────────────────────┴────────────────────────┘

Based on https://github.com/astral-sh/ruff/pull/9031
"""

import json
import os
import subprocess
import shutil
import tempfile
from pathlib import Path
import re
import argparse
from typing import NamedTuple

from rich import print as _rich_print
from rich.table import Table
from rich.console import Console


def _get_profile_name(lto_option: str | bool, codegen_unit: int) -> str:
    return f"{lto_option if isinstance(lto_option, str) else 'nolto'}cg{codegen_unit}"


# Configuration for benchmarking
CODEGEN_UNITS = [1, 16]
LTO_OPTIONS: list[str | bool] = ["fat", "thin", False]
PROFILES = [
    _get_profile_name(lto_option, codegen_unit)
    for lto_option in LTO_OPTIONS
    for codegen_unit in CODEGEN_UNITS
]
BENCHMARK_ARGS = "--profile django --line-length 120 --silent"
WARMUP_RUNS = 10
BENCHMARK_RUNS = 100
CARGO_TOML_PATH = Path("Cargo.toml")


class BuildResult(NamedTuple):
    binary_size_mb: float
    wheel_size_mb: float
    build_time_seconds: float


class BenchResult(NamedTuple):
    mean: float
    stddev: float


def rich_print(msg: str):
    msg = f" {msg} ".center(80, "=")
    _rich_print(f"[bold cyan]{msg}[/bold cyan]")


def update_cargo_toml(initial_cargo_toml_content: str):
    """Create a temporary Cargo.toml based on the original"""
    rich_print("Setting up temporary Cargo.toml...")

    if "[profile.fatcg1]" in initial_cargo_toml_content:
        rich_print("Profiles already set-up in Cargo.toml")
    else:
        with open("Cargo.toml", "a") as f:
            f.write(_generate_additional_profiles())
        rich_print("Profiles added to Cargo.toml")

    return initial_cargo_toml_content


def _generate_additional_profiles():
    profile_strings = []
    for codegen_unit in CODEGEN_UNITS:
        for lto_option in LTO_OPTIONS:
            profile_str = (
                f"[profile.{_get_profile_name(lto_option, codegen_unit)}]\n"
                f'inherits = "release"\n'
                f"lto = {repr(lto_option).lower()}\n"
                f"codegen-units = {codegen_unit}\n"
            )
            profile_strings.append(profile_str)

    return "\n".join(profile_strings)


def _get_file_size_mb(file_path):
    """Get file size in MB"""
    size_bytes = os.path.getsize(file_path)
    return size_bytes / (1024 * 1024)  # Convert to MB


def _parse_build_time_seconds(output):
    """Parse build time from cargo output"""
    match = re.search(r"Finished .+ in (\d+\.\d+)s", output)
    if match:
        return float(match.group(1))
    return None


def _sanity_checks():
    """Check for required tool and files"""
    for tool in ["cargo", "maturin", "hyperfine"]:
        tool_path = shutil.which(tool)
        if not tool_path:
            rich_print(
                f"Error: {tool} not found. Please install it before running this script."
            )
            raise SystemExit(1)

    if not CARGO_TOML_PATH.exists():
        raise FileNotFoundError(
            "Cargo.toml not found. Please run this script from your project root."
        )


def build_binaries() -> dict[str, BuildResult]:
    """Build all binaries and wheels, measuring build time"""
    rich_print("Building binaries and wheels for each profile")

    # Clean target directory
    subprocess.run(["cargo", "clean"], check=True)

    # Create target/release directory
    os.makedirs("target/release", exist_ok=True)

    results = {}

    for profile in PROFILES:
        rich_print(f":wrench: Building profile: {profile}")

        # Build binary and get file size
        process = subprocess.run(
            ["cargo", "build", "--profile", profile],
            capture_output=True,
            text=True,
            check=True,
        )
        binary_src = f"target/{profile}/djangofmt"
        binary_dest = f"target/release/djangofmt-{profile}"
        subprocess.run(["cp", binary_src, binary_dest])
        # shutil.copyfile(binary_src, binary_dest)
        # os.chmod(binary_dest, 0o777)

        # Parse build time
        build_time_seconds = _parse_build_time_seconds(process.stderr)
        if build_time_seconds is None:
            rich_print(f"Warning: Could not parse build time for profile {profile}")
            build_time_seconds = 0

        # Build wheel
        subprocess.run(
            [
                "maturin",
                "build",
                "--release",
                "--profile",
                profile,
            ],
            check=True,
        )
        # Find the wheel file
        wheel_files = list(Path("target/wheels").glob("djangofmt*.whl"))
        if wheel_files:
            latest_wheel = wheel_files[0]
            wheel_dest = f"target/release/djangofmt-{profile}.whl"
            shutil.copyfile(str(latest_wheel), wheel_dest)
            wheel_size_mb = _get_file_size_mb(wheel_dest)
        else:
            rich_print(f"Warning: No wheel found for profile {profile}")
            wheel_size_mb = 0

        # Store results
        results[profile] = BuildResult(
            binary_size_mb=_get_file_size_mb(binary_dest),
            wheel_size_mb=wheel_size_mb,
            build_time_seconds=build_time_seconds,
        )

    return results


def run_benchmarks(files_list_path) -> dict[str, BenchResult]:
    """Run benchmarks for all profiles and collect results"""
    rich_print("Running benchmarks")
    if not os.path.exists(files_list_path):
        raise FileNotFoundError(f"Files list not found at: {files_list_path}")

    with tempfile.NamedTemporaryFile() as f:
        benchmark_cmd = [
            "hyperfine",
            "-i",
            "--warmup",
            str(WARMUP_RUNS),
            "--runs",
            str(BENCHMARK_RUNS),
            "--export-json",
            f.name,
            "--show-output",
        ]
        for profile in PROFILES:
            cmd_str = f'"cat {files_list_path} | xargs --max-procs=0 ./target/release/djangofmt-{profile} {BENCHMARK_ARGS}"'
            benchmark_cmd.append(cmd_str)

        subprocess.run(" ".join(benchmark_cmd), check=True, shell=True)

        results = json.loads(f.read())["results"]

        return {
            profile: BenchResult(result["mean"], result["stddev"])
            for profile, result in zip(PROFILES, results, strict=True)
        }


def output_rich_table(
    build_results: dict[str, BuildResult], bench_results: dict[str, BenchResult]
):
    """Generate a formatted rich table with all results"""
    table = Table()

    # Add headers
    table.add_column("Profile", style="cyan")
    table.add_column("Build Time (s)", justify="right")
    table.add_column("Binary Size (MB)", justify="right")
    table.add_column("Wheel Size (MB)", justify="right")
    table.add_column("Benchmark Time Avg (ms)", justify="right")
    table.add_column("Benchmark Time ±σ (ms)", justify="right")

    # Add content
    table_data = []
    for profile in PROFILES:
        build_info = build_results[profile]

        row = [
            profile,
            f"{build_info.build_time_seconds:.2f}",
            f"{build_info.binary_size_mb:.2f}",
            f"{build_info.wheel_size_mb:.2f}",
            f"{bench_results[profile].mean * 1000:.2f}",
            f"{bench_results[profile].stddev * 1000:.2f}",
        ]

        table_data.append(row)

    # Sort by benchmark time (fastest first)
    table_data.sort(key=lambda x: float(x[4]))

    # Add rows to table
    for row in table_data:
        table.add_row(*row)

    console = Console()
    console.print(table)


def parse_arguments():
    """Parse command line arguments"""
    parser = argparse.ArgumentParser(
        description="Build and benchmark different cargo profiles"
    )
    parser.add_argument(
        "--files-list",
        default="/tmp/files-list",
        help="Path to a file containing a list of html files path to format for benchmarking",
    )
    return parser.parse_args()


def main():
    args = parse_arguments()

    _sanity_checks()

    initial_cargo_toml_content = CARGO_TOML_PATH.read_text()
    try:
        update_cargo_toml(initial_cargo_toml_content)

        # Build binaries and collect results
        build_results = build_binaries()

        # Run benchmarks
        bench_results = run_benchmarks(args.files_list)

        # Output table
        output_rich_table(build_results, bench_results)

    finally:
        # Reset the cargo toml file to previous state
        CARGO_TOML_PATH.write_text(initial_cargo_toml_content)
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
