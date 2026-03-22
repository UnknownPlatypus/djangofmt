ecosystem_cache_dir := "/tmp/djangofmt-ecosystem-repos"


# List all commands
_default:
    @just --list  --unsorted

# Bootstrap your environment
[group('dev')]
bootstrap:
    uv tool install pre-commit
    pre-commit install
    uv sync

# Pre-merge request checks
[group('dev')]
pre-mr-check:
    SKIP=actionlint,renovate-config-validator pre-commit run -a
    maturin develop
    cargo clippy --all-targets --all-features
    cargo test --all-targets --all-features

# Build playground WASM package
[group('playground')]
playground-wasm-build:
    wasm-pack build --target web crates/djangofmt_wasm --out-dir ../../playground/pkg

# Run playground dev server (builds WASM first)
[group('playground')]
playground-dev: playground-wasm-build
    npm ci --prefix playground
    npm run dev --prefix playground

# Setup python benchmarks
[group('bench')]
setup-bench-py:
    cargo build --release -p djangofmt
    uv sync --project ./python/benchmarks -p 3.11
    npm ci --prefix ./python/benchmarks

# Run python benchmarks on a directory of templates
[working-directory: 'python/benchmarks']
[group('bench')]
bench-py dir: setup-bench-py
    uv run ./run_formatter.sh {{dir}}

# Run rust micro-benchmarks
[group('bench')]
bench-rs:
    cargo bench -p djangofmt_benchmark

# Benchmark dev vs system djangofmt on HTML files
[group('bench')]
benchmark-git-repo repo_path:
    #!/usr/bin/env bash
    set -euo pipefail

    REPO_DIR=$(realpath "{{repo_path}}")
    ROOT_DIR=$(pwd)
    TEMP_DIR="/tmp/benchmark-djangofmt-$(date +%Y-%m-%dT%H:%M:%S%Z)"
    DJANGOFMT_DEV="$ROOT_DIR/target/release/djangofmt"
    FILES_LIST="$TEMP_DIR/files.txt"

    # 1. Build release version
    echo "Building release version of djangofmt..."
    cargo build -p djangofmt --release

    # 2. Create clean environment & selectively copy HTML
    echo "Creating lean benchmark environment at $TEMP_DIR..."
    mkdir -p "$TEMP_DIR"

    # Use rsync to mirror ONLY the directory structure and .html files
    rsync -am --include='*.html' --include='*/' --exclude='*' "$REPO_DIR/" "$TEMP_DIR/"

    cd "$TEMP_DIR"

    # 3. Initialize a fresh git repo for the benchmark reset logic
    # This allows 'git checkout .' to work between hyperfine runs
    git init -q
    git add .
    git -c user.email="benchmark@local" -c user.name="benchmark" commit -m "initial" -q

    # 4. Generate the file list for xargs
    find . -type f -name "*.html" -print0 > "$FILES_LIST"
    FILE_COUNT=$(tr -cd '\0' < "$FILES_LIST" | wc -c)

    if [ "$FILE_COUNT" -eq 0 ]; then
        echo "Error: No HTML files found in $REPO_DIR"
        exit 1
    fi

    echo "Found $FILE_COUNT HTML files to benchmark"

    # 5. Setup commands
    DEV_CMD="xargs -0 \"$DJANGOFMT_DEV\" --profile django --line-length 120 < \"$FILES_LIST\""
    SYS_CMD="xargs -0 djangofmt --profile django --line-length 120 < \"$FILES_LIST\""
    # Get versions
    DEV_VERSION=$("$DJANGOFMT_DEV" --version | cut -d" " -f2)
    SYS_VERSION=$(djangofmt --version 2>/dev/null | cut -d" " -f2 || echo "not found")

    echo "Benchmarking dev ($DEV_VERSION) vs system ($SYS_VERSION)"

    # 6. Run benchmark
    # We reset files to unformatted state before every single run
    hyperfine --ignore-failure \
        --warmup 1 \
        --prepare "git checkout . -q" \
        "$DEV_CMD" \
        "$SYS_CMD"


# Run format ecosystem checks with custom baseline and comparison executables
[group('ecosystem-check')]
ecosystem-check baseline comparison *args:
    cargo build -p djangofmt
    uv run ecosystem-check format {{baseline}} {{comparison}} --cache-dir {{ecosystem_cache_dir}} {{args}}

# Run format ecosystem checks comparing debug build to system djangofmt
[group('ecosystem-check')]
ecosystem-check-dev:
    cargo build -p djangofmt
    uv run ecosystem-check format djangofmt "target/debug/djangofmt" --cache-dir {{ecosystem_cache_dir}}

# Run format ecosystem checks comparing djangofmt debug build to 'djade' or 'rustywind'
[group('ecosystem-check')]
[arg('external-formatter', pattern='djade|rustywind')]
ecosystem-check-stability external-formatter:
    cargo build -p djangofmt
    uv run ecosystem-check format {{external-formatter}} "target/debug/djangofmt" --cache-dir {{ecosystem_cache_dir}} --format-comparison base-then-comp-converge

# Run check (lint) ecosystem checks with custom baseline and comparison executables
[group('ecosystem-check')]
ecosystem-check-lint baseline comparison *args:
    cargo build -p djangofmt
    uv run ecosystem-check check {{baseline}} {{comparison}} --cache-dir {{ecosystem_cache_dir}} {{args}}

# Run check (lint) ecosystem checks comparing debug build to system djangofmt
[group('ecosystem-check')]
ecosystem-check-lint-dev:
    cargo build -p djangofmt
    uv run ecosystem-check check djangofmt "target/debug/djangofmt" --cache-dir {{ecosystem_cache_dir}}

# Clean ecosystem check git repos cache
[group('ecosystem-check')]
ecosystem-check-clean-cache:
    rm -rf {{ecosystem_cache_dir}}
