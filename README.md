# Djangofmt

<!-- Begin section: Overview -->

[![Pypi Version](https://img.shields.io/pypi/v/djangofmt.svg)](https://pypi.python.org/djangofmt)
[![License](https://img.shields.io/pypi/l/djangofmt.svg)](https://github.com/UnknownPlatypus/djangofmt/blob/main/LICENSE)
[![Supported Python Versions](https://img.shields.io/pypi/pyversions/djangofmt.svg)](https://pypi.python.org/pypi/djangofmt)
[![Actions status](https://github.com/UnknownPlatypus/djangofmt/actions/workflows/ci.yml/badge.svg)](https://github.com/UnknownPlatypus/djangofmt/actions)
[![pre-commit.ci status](https://results.pre-commit.ci/badge/github/UnknownPlatypus/djangofmt/main.svg)](https://results.pre-commit.ci/latest/github/UnknownPlatypus/djangofmt/main)
[![CodSpeed Badge](https://img.shields.io/endpoint?url=https://codspeed.io/badge.json)](https://codspeed.io/UnknownPlatypus/djangofmt?utm_source=badge)

[**Docs**](https://unknownplatypus.github.io/djangofmt/docs/) | [**Playground**](https://unknownplatypus.github.io/djangofmt/)

A fast, HTML aware, Django template formatter, written in Rust.

<p align="center">
  <picture align="center">
    <source media="(prefers-color-scheme: dark)" srcset="https://github.com/user-attachments/assets/3b09a8a2-b5cb-4f1b-a0bc-5f4e3ca169db">
    <source media="(prefers-color-scheme: light)" srcset="https://github.com/user-attachments/assets/88dda91e-cfdd-45a7-a3b4-1f3cc2d0fe95">
    <img alt="Shows a bar chart with benchmark results." src="https://github.com/user-attachments/assets/88dda91e-cfdd-45a7-a3b4-1f3cc2d0fe95" style="max-width: 75%;">
  </picture>
</p>

<p align="center">
  <i>Formatting 100k+ lines of HTML across 1.7k+ files from scratch.</i>
</p>

Heavily rely on the awesome [markup_fmt](https://github.com/g-plane/markup_fmt) with some additions to support Django fully.

## Table of contents

- [Installation](#installation)
- [Usage](#usage)
- [Pre-commit hook](#pre-commit-hook)
- [Configuration](#configuration)
- [Editor integration](https://unknownplatypus.github.io/djangofmt/docs/editor-integration/)
- [Controlling the formatting](https://unknownplatypus.github.io/djangofmt/docs/formatting/)
- [Lint rules](https://unknownplatypus.github.io/djangofmt/docs/rules/)
- [Known limitations](https://unknownplatypus.github.io/djangofmt/docs/known-limitations/)
- [Benchmarks](https://unknownplatypus.github.io/djangofmt/docs/benchmarks/)
- [Shell completions](#shell-completions)
- [Contributing](#contributing)

## Installation

djangofmt is available on PyPI.

```shell
# With pip
pip install djangofmt

# With uv
uv tool install djangofmt@latest  # Install djangofmt globally.
uv add --dev djangofmt            # Or add djangofmt to your project.

# With pipx
pipx install djangofmt
```

## Usage

```shell
djangofmt .                    # Format all files in the current directory (and any subdirectories).
djangofmt src/templates        # Format all template files in `src/templates`
djangofmt templates/base.html  # Format individual files
```

When given a directory, djangofmt recurses into it and formats all `*.html`, `*.jinja`, `*.jinja2`, and `*.j2` files it finds.
It also respects `.gitignore` files.

### Looking for a check mode ?

djangofmt intentionally does not provide a built-in check functionality because CI is too late for a code formatter. We strongly recommend using pre-commit or any IDE "format on save" integration. That being said, you can emulate check capability by chaining with a git diff command like so:

```bash
djangofmt .
git diff --exit-code -- '*.html' || (echo "HTML templates are not formatted. Run 'djangofmt' to fix." && exit 1)
```

## Pre-commit hook

See [pre-commit](https://github.com/pre-commit/pre-commit) for instructions.

Sample `.pre-commit-config.yaml`:

```yaml
- repo: https://github.com/UnknownPlatypus/djangofmt-pre-commit
  rev: v0.2.10
  hooks:
    - id: djangofmt
```

The [separate repository](https://github.com/UnknownPlatypus/djangofmt-pre-commit) enables installation without compiling the Rust code.

By default, the configuration uses pre-commit's [`files` option](https://pre-commit.com/#creating-new-hooks) to detect
all text files in directories named `templates`. If your templates are stored elsewhere, you can override this behavior
by specifying the desired files in the hook configuration within your `.pre-commit-config.yaml` file.

### `.svg` files support

djangofmt can format svg files too. It will behave exactly the same way as if they were html files.
There is a dedicated pre-commit hook for these:

```yaml
- repo: https://github.com/UnknownPlatypus/djangofmt-pre-commit
  rev: v0.2.10
  hooks:
    - id: djangofmt-svg
```

## Configuration

Djangofmt can also be configured via a `[tool.djangofmt]` section in your `pyproject.toml`:

```toml
[tool.djangofmt]
line-length = 120
indent-width = 4
profile = "django"
custom-blocks = ["stage", "flatblock"]
html-void-self-closing = "never"
preserve-unquoted-attrs = false
```

Djangofmt looks for a `pyproject.toml` file by traversing directories upward from the current working directory.
The first `pyproject.toml` found is used. If no file is found or the file doesn't contain a `[tool.djangofmt]` section, defaults are used.

Command-line arguments always take precedence over `pyproject.toml` settings.

See [Controlling the formatting](https://unknownplatypus.github.io/djangofmt/docs/formatting/) for the behaviour of each option and how to opt into per-node overrides.

## Editor integration

See the [editor integration guide](https://unknownplatypus.github.io/djangofmt/docs/editor-integration/).

## Shell completions

You can generate shell completions for your preferred
shell using the `djangofmt completions` command.

```shell
Usage: djangofmt completions <SHELL>

Arguments:
  <SHELL>
      The shell to generate the completions for
      [possible values: bash, elvish, fish, nushell, powershell, zsh]
```

## Contributing

Contributions are welcome! Please see [`CONTRIBUTING.md`](CONTRIBUTING.md) for details on how to get started.

<!-- End section: Overview -->
