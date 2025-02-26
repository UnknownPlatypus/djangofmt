# Djangofmt

A fast, HTML aware, Django template formatter, written in Rust.

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

## As a pre-commit hook

See [pre-commit](https://github.com/pre-commit/pre-commit) for instructions

Sample `.pre-commit-config.yaml`:

```yaml
- repo: https://github.com/UnknownPlatypus/djangofmt-pre-commit
  rev: v0.1.0
  hooks:
    - id: djangofmt
```

The separate repository enables installation without compiling the Rust code.

By default, the configuration uses pre-commit’s [`files` option](https://pre-commit.com/#creating-new-hooks) to detect
all text files in directories named `templates`. If your templates are stored elsewhere, you can override this behavior
by specifying the desired files in the hook configuration within your `.pre-commit-config.yaml` file.

## Usage

```shell
Usage: djangofmt [OPTIONS] <FILES>...

Arguments:
  <FILES>...
          List of files to format

Options:
      --line-length <LINE_LENGTH>
          Set the line-length [default: 120]
      --profile <PROFILE>
          Template language profile to use [default: django] [possible values: django, jinja]
      --custom-blocks <BLOCK_NAMES>
          Comma-separated list of custom block name to enable
  -h, --help
          Print help
  -V, --version
          Print version
```
