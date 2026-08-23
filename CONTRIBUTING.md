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

## Fuzzing

The formatter can be fuzzed with [cargo-fuzz](https://github.com/rust-fuzz/cargo-fuzz)
(requires `rustup toolchain install nightly` and `cargo install cargo-fuzz`):

```shell
just fuzz     # run until interrupted
just fuzz 60  # run for 60 seconds
```

The `fmt_idempotency` target formats arbitrary inputs twice under both the Django
and the Jinja profile, and panics on any panic inside the formatter, non-idempotent
output, or output that no longer parses. It is seeded with the in-repo test fixtures
and benchmark templates.

When a crash is found (an input file lands in `fuzz/artifacts/fmt_idempotency/`):

1. Minimize it: `just fuzz-min fuzz/artifacts/fmt_idempotency/crash-...`
2. Reproduce and fix the bug in the [markup_fmt fork](https://github.com/UnknownPlatypus/markup_fmt),
   then bump the pinned `rev` in the root `Cargo.toml`.
3. Keep the minimized case as a fixture in `crates/djangofmt/tests/fmt/`
   or `crates/djangofmt/tests/parse_error/`.

## Use of AI

> [!NOTE]
> We **require all use of AI in contributions to follow the following policy**.
> If your contribution does not follow the policy, it will be closed.

We support using AI (i.e., LLMs) as tools for coding. However, you remain
responsible for any code you publish and we are responsible for any code we
merge and release. We hold a high bar for all contributions to our projects.

**AI should not be used to generate comments when communicating with
maintainers**. We expect comments on our projects to be written by humans. We
may hide any comments that we believe are AI generated.

If you are opening an issue, we expect you to describe the problem in your own
words.

If you are opening a pull request, we expect you to be able to explain the
proposed changes in your own words. This includes the pull request body and
responses to questions. **Do not copy responses from the AI when replying to
questions from maintainers.**

We require a human in the loop who understands the work produced by AI. **We do
not allow autonomous agents to be used for contributing to our projects**. We
will close any pull requests that we believe were created autonomously.

If you wish to include context from an interaction with AI in your comments, it
must be in a quote block (e.g., using `>`) and disclosed as such. It must be
accompanied by human commentary explaining the relevance and implications of the
context. Do not share long snippets.

We understand that AI is useful when communicating as a non-native English
speaker. If you are using AI to edit your comments for this purpose, please take
the time to ensure it reflects your own voice and ideas. If using AI for
translation, we recommend writing in your native language and including the AI
translation in a quote block.
