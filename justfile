# List all commands
default:
    @just --list

# Bootstrap your environment
bootstrap:
    uv tool install pre-commit
    pre-commit install
    uv sync

# Pre-merge request check
pre-mr-check:
    SKIP=actionlint,renovate-config-validator pre-commit run -a
    maturin develop
    cargo clippy --all-targets --all-features
    cargo test --all-targets --all-features

# Build playground WASM package
playground-wasm-build:
    wasm-pack build --target web crates/djangofmt_wasm --out-dir ../../playground/pkg

# Run playground dev server (builds WASM first)
playground-dev: playground-wasm-build
    npm ci --prefix playground
    npm run dev --prefix playground

# Setup python benchmarks
setup-bench-py:
    cargo build --release -p djangofmt
    uv sync --project ./python/benchmarks -p 3.11
    npm ci --prefix ./python/benchmarks

# Run python benchmarks on a directory of templates
[working-directory: 'python/benchmarks']
bench-py dir: setup-bench-py
    uv run ./run_formatter.sh {{dir}}

# Run rust micro-benchmarks
bench-rs:
    cargo bench -p djangofmt_benchmark

# Run ecosystem checks with custom baseline and comparison executables
ecosystem-check baseline comparison *args:
    cargo build -p djangofmt
    uv run ecosystem-check format {{baseline}} {{comparison}} --cache-dir /tmp/repos {{args}}

# Run ecosystem checks comparing debug build to system djangofmt
ecosystem-check-dev:
    cargo build -p djangofmt
    uv run ecosystem-check format djangofmt "target/debug/djangofmt" --cache-dir /tmp/repos

# Clean ecosystem check git repos cache
ecosystem-check-clean-cache:
    rm -rf /tmp/repos
