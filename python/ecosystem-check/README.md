# ecosystem-check

Compare format results for two different executable versions (e.g. main and a PR) on real world projects.

## Installation

From this project root, install with `pip`:

```shell
pip install -e ./python/ecosystem-check
```

## Usage

```shell
ecosystem-check format <baseline executable> <comparison executable>
```

Note executable paths may be absolute, relative to the current working directory, or will be looked up in the
current Python environment and PATH.

Run `djangofmt format` ecosystem checks comparing your debug build to your system djangofmt:

```shell
ecosystem-check format djangofmt "./target/debug/djangofmt"
```

Run `djangofmt format` ecosystem checks comparing with changes to code that is already formatted:

```shell
ecosystem-check format djangofmt "./target/debug/djangofmt" --format-comparison base-then-comp
```

The default output format is markdown, which includes nice summaries of the changes. You can use `--output-format json` to display the raw data — this is
particularly useful when making changes to the ecosystem checks.

## Development

When developing, it can be useful to set the `--pdb` flag to drop into a debugger on failure:

```shell
ecosystem-check format djangofmt "./target/debug/djangofmt" --pdb
```

You can also provide a path to cache checkouts to speed up repeated runs:

```shell
ecosystem-check format djangofmt "./target/debug/djangofmt" --cache /tmp/repos
```
