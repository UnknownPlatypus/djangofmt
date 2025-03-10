#!/usr/bin/env sh

TARGET_DIR=${1}

# Find template files & set max line length (default to 120)
find "$TARGET_DIR" -name "*.html"> /tmp/files-list
LINE_LENGTH=${LINE_LENGTH:-120}

# Setup run commands
_XARGS_FILES="cat /tmp/files-list | xargs --max-procs=0"
DJANGOFMT_CMD="$_XARGS_FILES ../../target/release/djangofmt --profile django --line-length $LINE_LENGTH --quiet"
PRETTIER_CMD="$_XARGS_FILES ./node_modules/.bin/prettier --ignore-unknown --write --print-width $LINE_LENGTH --log-level silent"
DJLINT_CMD="$_XARGS_FILES djlint --reformat --profile=django --max-line-length $LINE_LENGTH"
DJADE_CMD="$_XARGS_FILES djade --target-version 5.1"
DJHTML_CMD="$_XARGS_FILES djhtml"


printf "Running benchmark on %s files (%s LoC)...\n\nTool versions:\n" "$(wc -l /tmp/files-list | cut -d " " -f1)" "$(xargs wc -l < "$TARGET_DIR" | grep "\d* total" | cut -d " " -f2)"
printf "  - django-fmt: v%s\n" "$(../../target/release/djangofmt --version | cut -d" " -f2 )"
printf "  - prettier: v%s\n" "$(./node_modules/.bin/prettier --version)"
printf "  - djlint: v%s\n" "$(djlint --version | cut -d" " -f3)"
printf "  - djade: v%s\n" "$(djade --version | cut -d" " -f2)"
printf "  - djhtml: v%s\n\n" "$(djhtml --version)"

hyperfine --ignore-failure \
  --prepare "$DJANGOFMT_CMD || true" \
  "$DJANGOFMT_CMD" \
  --prepare "$DJADE_CMD || true" \
  "$DJADE_CMD" \
  --prepare "$DJHTML_CMD" \
  "$DJHTML_CMD" \
  --prepare "$DJLINT_CMD || true" \
  "$DJLINT_CMD" \
  --prepare "$PRETTIER_CMD || true" \
  "$PRETTIER_CMD"
