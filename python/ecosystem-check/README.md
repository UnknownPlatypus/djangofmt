# ecosystem-check

Compare format results for two different executable versions (e.g. main and a PR) on real world projects.

## Usage

```shell
uv run ecosystem-check format <baseline executable> <comparison executable>
```

Note executable paths may be absolute, relative to the current working directory, or will be looked up in the
current Python environment and PATH.

Run `djangofmt` ecosystem checks comparing your debug build to your system djangofmt:

```shell
uv run ecosystem-check format djangofmt "../../target/debug/djangofmt"
```

Run `djangofmt` ecosystem checks comparing with changes to code that is already formatted:

```shell
uv run ecosystem-check format djangofmt "../../target/debug/djangofmt" --format-comparison base-then-comp
```

The default output format is markdown, which includes nice summaries of the changes. You can use `--output-format json` to display the raw data — this is
particularly useful when making changes to the ecosystem checks.

## Development

When developing, it can be useful to set the `--pdb` flag to drop into a debugger on failure:

```shell
uv run ecosystem-check format djangofmt "../../target/debug/djangofmt" --pdb
```

You can also provide a path to cache checkouts to speed up repeated runs:

```shell
uv run ecosystem-check format djangofmt "../../target/debug/djangofmt" --cache-dir /tmp/repos
```
