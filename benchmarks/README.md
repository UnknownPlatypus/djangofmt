# Benchmarks

Documentation about how to run various performance benchmark for `djangofmt` versus similar tools:

- `djlint`: Same scope as `djangofmt` - fully featured template formatter
- `djhtml`: Only an indenter, it will never add/remove newlines
- `djade`: Only format django template syntax - HTML is not formatted
- `prettier`: Does not support Django and only format HTML

## Setup

First go into `./benchmarks` and then run this to build `djangofmt` & install other tools needed to benchmark:

```bash
cargo build --release &&
uv sync --project . -p 3.11 &&
. .venv/bin/activate &&
npm i
```

## Running Benchmarks

Simply run this command, providing a directory containing django templates.

```bash
./run_formatter.sh ~/templates
```

A setup step will discover every html files inside and the run the various tools on it.
This will cause destructive operations, be sure to target a safe directory (tracked with git or temporary)
You can change the print width with the `LINE_LENGTH` env variable (default: 120)
