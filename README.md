# Djangofmt

[![Pypi Version](https://img.shields.io/pypi/v/djangofmt.svg)](https://pypi.python.org/djangofmt)
[![License](https://img.shields.io/pypi/l/djangofmt.svg)](https://github.com/UnknownPlatypus/djangofmt/blob/main/LICENSE)
[![Supported Python Versions](https://img.shields.io/pypi/pyversions/djangofmt.svg)](https://pypi.python.org/pypi/djangofmt)
[![Actions status](https://github.com/UnknownPlatypus/djangofmt/actions/workflows/ci.yml/badge.svg)](https://github.com/UnknownPlatypus/djangofmt/actions)
[![pre-commit.ci status](https://results.pre-commit.ci/badge/github/UnknownPlatypus/djangofmt/main.svg)](https://results.pre-commit.ci/latest/github/UnknownPlatypus/djangofmt/main)

A fast, HTML aware, Django template formatter, written in Rust.

Heavily rely on the awesome [markup_fmt](https://github.com/g-plane/markup_fmt) with some additions to support Django fully.

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
  rev: v0.2.0
  hooks:
    - id: djangofmt
```

The [separate repository](https://github.com/UnknownPlatypus/djangofmt-pre-commit) enables installation without compiling the Rust code.

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

Djangofmt does not have any ability to recurse through directories.
Use the pre-commit integration, globbing, or another technique to apply it to many files.
For example, [git ls-files | xargs](https://adamj.eu/tech/2022/03/09/how-to-run-a-command-on-many-files-in-your-git-repository/):

```shell
git ls-files -z -- '*.html' | xargs -0r djangofmt
```

…or PowerShell’s [`ForEach-Object`](https://learn.microsoft.com/en-us/powershell/module/microsoft.powershell.core/foreach-object):

```shell
git ls-files -- '*.html' | %{djangofmt $_}
```

## Controlling the formatting

DjangoFmt gives users control over formatting in cases where
static analysis struggles to determine the optimal approach.

### Splitting an opening tag across multiple lines

You can control this formatting by choosing whether to insert a newline before the first attribute:

```diff
# Unchanged
<div class="flex" id="great" data-a>
  This is nice!
</div>

# Wrap on multiple lines
<div
-    class="flex" id="great" data-a>
+    class="flex"
+    id="great"
+    data-a
+>
    This is nice!
</div>
```

### Class attribute formatting

The `class` attribute will be formatted as a space-separated sequence of strings,
unless there are already newlines inside the attribute value.

This makes it possible to accommodate the 2 following use cases:

```html
<div class="
  mt-8 p-8
  bg-indigo-600 hover:bg-indigo-700
  border border-transparent
  font-medium text-white
">
    Hello world
</div>

<div class="mt-8 p-8 bg-indigo-600 hover:bg-indigo-700 border border-transparent font-medium text-white">
    Hello world
</div>
```

See https://github.com/g-plane/markup_fmt/issues/75#issuecomment-2456526352 for the rationale.

## Known limitations

All of these could be solved but were not prioritized for now.

### `style` attributes formatting

The `style` attribute will be formatted using a CSS formatter ([Malva](https://github.com/g-plane/malva)),
but the output will always be on a single line.

**Before:**

```html
<div class="flex flex-col items-center absolute z-10"
     style="top:60%;
            transform:translate(0,-50%)">
    Such a lovely day
</div>
```

**After:**

```html
<div class="flex flex-col items-center absolute z-10"
     style="top:60%; transform:translate(0,-50%)">
    Such a lovely day
</div>
```

## Benchmarks

Here are the results benchmarking `djangofmt` against similar tools on 100k lines of HTML across 1.7k files.

<p align="center">
  <picture align="center">
    <source media="(prefers-color-scheme: dark)" srcset="https://github.com/user-attachments/assets/3b09a8a2-b5cb-4f1b-a0bc-5f4e3ca169db">
    <source media="(prefers-color-scheme: light)" srcset="https://github.com/user-attachments/assets/88dda91e-cfdd-45a7-a3b4-1f3cc2d0fe95">
    <img alt="Shows a bar chart with benchmark results." src="https://github.com/user-attachments/assets/88dda91e-cfdd-45a7-a3b4-1f3cc2d0fe95">
  </picture>
</p>

<p align="center">
  <i>Formatting 100k+ lines of HTML across 1.7k+ files from scratch.</i>
</p>

This is important to note that only `djlint` covers the same scope in terms of formatting capabilities.
`djade` only alter django templating, `djhtml` only fix indentation and `prettier` only understand html (and **will** break templates)

As always, these results should be taken with a grain of salt.
Results on my machine will differ from yours, especially if you have many CPU cores because some tools take better advantage of parallelization than others.

But at least it was fun to build thanks to the wonderful [hyperfine](https://github.com/sharkdp/hyperfine) tool.

<details>
  <summary>Benchmark details (2025-02-28)</summary>

This was run on my AMD Ryzen 9 7950X (32) @ 5.881GHz.

Tools versions:

- djangofmt: v0.1.0
- prettier: v3.5.2
- djlint: v1.36.4
- djade: v1.3.2
- djhtml: v3.0.7

<pre><code>Benchmark 1: cat /tmp/test-files | xargs --max-procs=0 ../../target/release/djangofmt format --profile django --line-length 120 --quiet
  Time (mean ± σ):      19.8 ms ±   0.9 ms    [User: 179.6 ms, System: 73.7 ms]
  Range (min … max):    18.3 ms …  23.3 ms    73 runs

  Warning: Ignoring non-zero exit code.

Benchmark 2: cat /tmp/test-files | xargs --max-procs=0 djade --target-version 5.1
  Time (mean ± σ):      72.0 ms ±   1.0 ms    [User: 63.2 ms, System: 9.3 ms]
  Range (min … max):    70.5 ms …  73.4 ms    18 runs

Benchmark 3: cat /tmp/test-files | xargs --max-procs=0 djhtml
  Time (mean ± σ):      1.401 s ±  0.026 s    [User: 1.322 s, System: 0.079 s]
  Range (min … max):    1.373 s …  1.453 s    10 runs

Benchmark 4: cat /tmp/test-files | xargs --max-procs=0 djlint --reformat --profile=django --max-line-length 120
  Time (mean ± σ):      2.343 s ±  0.026 s    [User: 64.944 s, System: 1.176 s]
  Range (min … max):    2.297 s …  2.377 s    10 runs

  Warning: Ignoring non-zero exit code.

Benchmark 5: cat /tmp/test-files | xargs --max-procs=0 ./node_modules/.bin/prettier --ignore-unknown --write --print-width 120 --log-level silent
  Time (mean ± σ):      3.226 s ±  0.062 s    [User: 4.481 s, System: 0.261 s]
  Range (min … max):    3.092 s …  3.292 s    10 runs

  Warning: Ignoring non-zero exit code.

Summary
  cat /tmp/test-files | xargs --max-procs=0 ../../target/release/djangofmt format --profile django --line-length 120 --quiet ran
    3.63 ± 0.17 times faster than cat /tmp/test-files | xargs --max-procs=0 djade --target-version 5.1
   70.71 ± 3.45 times faster than cat /tmp/test-files | xargs --max-procs=0 djhtml
  118.28 ± 5.48 times faster than cat /tmp/test-files | xargs --max-procs=0 djlint --reformat --profile=django --max-line-length 120
  162.80 ± 7.96 times faster than cat /tmp/test-files | xargs --max-procs=0 ./node_modules/.bin/prettier --ignore-unknown --write --print-width 120 --log-level silent
</code></pre>
</details>

## Shell Completions

You can generate shell completions for your preferred
shell using the `djangofmt completions` command.

```shell
Usage: djangofmt completions <SHELL>

Arguments:
  <SHELL>
      The shell to generate the completions for
      [possible values: bash, elvish, fish, nushell, powershell, zsh]
```
