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
playground-build:
    wasm-pack build --target web crates/djangofmt_wasm --out-dir ../../playground/pkg

# Run playground dev server (builds WASM first)
playground-dev: playground-build
    npm run dev --prefix playground

# Setup python benchmarks
setup-bench-py:
    cargo build --release
    uv sync --project ./python/benchmarks -p 3.11
    npm install --prefix ./python/benchmarks

# Run python benchmarks on a directory of templates
[working-directory: 'python/benchmarks']
bench-py dir: setup-bench-py
    uv run ./run_formatter.sh {{dir}}

# Run rust micro-benchmarks
bench-rs:
    cargo bench -p djangofmt_benchmark
