ecosystem_cache_dir := "/tmp/djangofmt-ecosystem-repos"
export INSTA_UPDATE := "always"

# List all commands
_default:
    @just --list  --unsorted

# Bootstrap your environment
[group('dev')]
bootstrap:
    uv tool install pre-commit
    pre-commit install
    uv sync
    rustup component add llvm-tools-preview
    cargo install cargo-llvm-cov --locked

# Run clippy on all targets and features
[group('dev')]
lint:
    cargo clippy --all-targets --all-features

# Run the full test suite, accepting snapshot updates automatically.
[group('dev')]
test:
    cargo test --workspace --all-targets --all-features

# Pre-merge request checks
[group('dev')]
pre-mr-check:
    SKIP=actionlint,renovate-config-validator pre-commit run -a
    uv run maturin develop
    just lint
    just test
    just docs-build

# Regenerate the per-rule documentation under docs/rules/ and docs/rules.md
# (and sync README/CONTRIBUTING into docs/).
[group('docs')]
docs-generate:
    cargo run -p djangofmt_dev -- generate-all

# Build the Zensical docs site into `site/` (regenerates docs first).
# `--isolated` runs zensical in a throwaway venv, which skips the maturin
# compile of the djangofmt wheel (the docs site doesn't need it) and leaves
# the project's `.venv` untouched.
[group('docs')]
docs-build: docs-generate
    uv run --isolated --group docs zensical build --clean --strict --config-file .mkdocs.yml

# Serve the Zensical docs site with live-reload (regenerates docs first).
[group('docs')]
docs-serve: docs-generate
    uv run --isolated --group docs zensical serve --config-file .mkdocs.yml

# Build playground WASM package
[group('playground')]
playground-wasm-build:
    wasm-pack build --target web crates/djangofmt_wasm --out-dir ../../playground/pkg

# Run playground dev server (builds WASM first)
[working-directory: 'playground']
[group('playground')]
playground-dev: playground-wasm-build
    deno install
    deno task dev

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

# Generate HTML coverage report and open in browser
[group('dev')]
coverage:
    cargo llvm-cov --workspace --exclude djangofmt_dev --html --open

# Generate LCOV coverage report
[group('dev')]
coverage-lcov:
    cargo llvm-cov --workspace --exclude djangofmt_dev --lcov --output-path lcov.info

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


# Run formatter ecosystem checks
[group('ecosystem-check')]
ecosystem-check baseline comparison *args:
    cargo build -p djangofmt
    uv run ecosystem-check format {{baseline}} {{comparison}} --cache-dir {{ecosystem_cache_dir}} {{args}}

# Run formatter ecosystem checks comparing debug build to system djangofmt
[group('ecosystem-check')]
ecosystem-check-dev:
    cargo build -p djangofmt
    uv run ecosystem-check format djangofmt "target/debug/djangofmt" --cache-dir {{ecosystem_cache_dir}}

# Run formatter ecosystem checks comparing djangofmt debug build to 'djade' or 'rustywind'
[group('ecosystem-check')]
[arg('external-formatter', pattern='djade|rustywind')]
ecosystem-check-stability external-formatter:
    cargo build -p djangofmt
    uv run ecosystem-check format {{external-formatter}} "target/debug/djangofmt" --cache-dir {{ecosystem_cache_dir}} --format-comparison base-then-comp-converge

# Run linter ecosystem checks
[group('ecosystem-check')]
ecosystem-check-lint baseline comparison *args:
    cargo build -p djangofmt
    uv run ecosystem-check check {{baseline}} {{comparison}} --cache-dir {{ecosystem_cache_dir}} {{args}}

# Run linter ecosystem checks comparing debug build to system djangofmt
[group('ecosystem-check')]
ecosystem-check-lint-dev:
    cargo build -p djangofmt
    uv run ecosystem-check check djangofmt "target/debug/djangofmt" --cache-dir {{ecosystem_cache_dir}}

# Clean ecosystem check git repos cache
[group('ecosystem-check')]
ecosystem-check-clean-cache:
    rm -rf {{ecosystem_cache_dir}}
