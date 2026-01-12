# Contributing to djangofmt

## Prerequisites

Djangofmt is written in Rust, so you'll need to install the
[Rust toolchain](https://www.rust-lang.org/tools/install).

You will also need [uv](https://docs.astral.sh/uv/getting-started/installation/) to run various python tools
and [`just`](https://github.com/casey/just#installation) to run development commands.

## Development

To get started, bootstrap your environment:

```shell
just bootstrap
```

This will install all the necessary tools and dependencies.

Before opening a pull request, make sure all checks are passing by running:

```shell
just pre-mr-check
```

This will run linting, formatting, and tests.

## Other tools / scripts

You can run the benchmarks locally using `just`.

To run the Python-based comparative benchmarks:

```shell
just bench-py <path/to/templates>
```

See the [benchmark README](./python/benchmarks/README.md) for more details.

To run the Rust-based micro-benchmarks:

```shell
just bench-rs
```

You can also run the ecosystem check locally (it will run in CI anyway).

```shell
just ecosystem-check-dev
```

See the [README](./python/ecosystem-check/README.md).
