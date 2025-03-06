# Contributing to djangofmt

## Prerequisites

Djangofmt is written in Rust, so you'll need to install the
[Rust toolchain](https://www.rust-lang.org/tools/install).

You will also need [uv](https://docs.astral.sh/uv/getting-started/installation/) (or [pipx](https://github.com/pypa/pipx))
to run various python tools.

Linting & formatting is managed using [pre-commit](https://pre-commit.com/)
so you'll need to have it installed and to install the hooks.

```shell
uv tool install pre-commit
pre-commit install
```

## Development

Before opening a pull request, make sure build / linting / formatting and the pass.

```shell
cargo build --release &&
pre-commit run --all-files
```

You can also run the ecosystem check locally (it will run in CI anyways).
See the [README](./python/ecosystem-check/README.md).

## Other tools / scripts

You can run the benchmarks locally followingthe [README](./python/benchmarks/README.md).
