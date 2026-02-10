#!/usr/bin/env bash

set -e

if ! command -v typos &>/dev/null; then
	echo "typos is not installed. Run 'cargo install typos-cli' to install it, otherwise the typos won't be fixed"
fi

if [ -z "$1" ]; then
	echo "Please provide a tag."
	echo "Usage: ./release.sh v[X.Y.Z]"
	exit
fi

echo "Preparing $1..."
# update the version in various files
sed -E -i "s/^version = .*$/version = \"${1#v}\"/" Cargo.toml pyproject.toml
sed -E -i "s/rev: v.*$/rev: v${1#v}/" README.md
sed -E -i "s/(djangofmt) [0-9]+\.[0-9]+\.[0-9]+/\1 ${1#v}/" crates/djangofmt/src/args.rs
sed -E -i "s/placeholder: \"[0-9]+\.[0-9]+\.[0-9]+\"$/placeholder: \"${1#v}\"/" .github/ISSUE_TEMPLATE/bug_report.yml
# sync cargo.lock
cargo build
# update the changelog
git cliff --tag "$1" -o
git add -A && pre-commit run || true
git add -A && pre-commit run

# generate a changelog for the tag message
export GIT_CLIFF_TEMPLATE="\
	{% for group, commits in commits | group_by(attribute=\"group\") %}
	{{ group | upper_first }}\
	{% for commit in commits %}
		- {% if commit.breaking %}(breaking) {% endif %}{{ commit.message | upper_first }} ({{ commit.id | truncate(length=7, end=\"\") }})\
	{% endfor %}
	{% endfor %}"
changelog=$(git cliff --unreleased --strip all)

# prompt before this
echo -n "Ready to commit and tag release $1? [y/N] "
read -r response
if [[ ! "$response" =~ ^[yY]([eE][sS])?$ ]]; then
	echo "Release cancelled."
	exit 1
fi

git commit -m "chore(release): prepare for $1"
git tag -a "$1" -m "Release $1" -m "$changelog"
git tag -v "$1"
echo "Done!"
echo "Now push the commit (git push) and the tag (git push --tags)."
