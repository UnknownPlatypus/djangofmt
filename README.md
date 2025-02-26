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
