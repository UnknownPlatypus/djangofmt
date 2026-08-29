#!/usr/bin/env sh

# Render chart.vl.json to SVG in both GitHub color schemes.
set -e
OUT_DIR=${1:-.}

render() {
	node -e '
		const spec = require("./chart.vl.json");
		spec.config.params.find((p) => p.name === "labelColor").value = process.argv[1];
		process.stdout.write(JSON.stringify(spec));
	' "$2" | ./node_modules/.bin/vl2svg >"$OUT_DIR/benchmark-$1.svg"
	printf "  %s\n" "$OUT_DIR/benchmark-$1.svg"
}

printf "Rendered:\n"
render light '#333333'
render dark '#c9d1d9'
