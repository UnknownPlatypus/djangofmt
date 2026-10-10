---
name: babysit-pr
description: Drive an open djangofmt PR to green — conflicts, review threads, CodSpeed and ecosystem-check verdicts.
disable-model-invocation: true
---

# Babysit PR

Takes a PR number; default is the current branch's PR. Work through the four checks, then push **once** so CI (and CodSpeed, which is the slow one) runs a single time.

## 1. Conflicts

`gh pr view N --json mergeable,mergeStateStatus`. `CONFLICTING` → `git rebase origin/main`, resolve, re-run the tests the conflict touched. `MERGEABLE` → leave the branch alone, even when behind `main`.

Done when `mergeable` is `MERGEABLE`.

## 2. Review threads

`gh pr view` doesn't expose review threads; use GraphQL:

```bash
gh api graphql -f query='query{repository(owner:"UnknownPlatypus",name:"djangofmt"){pullRequest(number:N){reviewThreads(first:50){nodes{isResolved path line comments(first:20){nodes{author{login} body}}}}}}}' --jq '.data.repository.pullRequest.reviewThreads.nodes[]|select(.isResolved|not)'
```

A `coderabbitai` "Review limit reached" issue comment is not a review. Address every unresolved thread in one commit.

Done when each unresolved thread is either fixed in that commit or answered in the thread with the reason it isn't.

## 3. CodSpeed

`gh pr checks N` — a failing `CodSpeed Performance Analysis` is a regression; the `codspeed` comment's **Performance Changes** table names the benchmarks. Its footnote may say the base is an older `main` commit: then the table includes `main` changes that aren't yours.

Triage from the code path before touching the profiler:

- Which profile runs each benchmark is in `crates/djangofmt_benchmark/src/lib.rs` (`jinja_large/*` is `Profile::Jinja`). A benchmark your change should not reach but which regressed means the new work runs unconditionally; gate it on the condition that makes it necessary.
- A single small benchmark a few percent off with every other one untouched is noise: acknowledge it on CodSpeed and say so in the PR.
- Anything else: `codspeed:codspeed-optimize`. The `codspeed` CLI is not installed (only `cargo-codspeed`), so measure through CI: push, then `compare_runs` from the CodSpeed MCP between the new run and the previous head, and `query_flamegraph` on the worst benchmark.

Done when the check is green, or the regression is acknowledged with its reason in the PR.

## 4. Ecosystem check

The `github-actions` comment `ecosystem-check results` is partial when large; the full diff is the linked workflow run. To reproduce locally: `just ecosystem-check <main-binary> target/debug/djangofmt`.

Every removed line must map to an added line through the PR's intended transformation. Classify the whole diff with a normalizing script (strip the intended change from both sides, then multiset-compare) rather than reading files; the residue is either line wraps caused by longer output (fine) or a regression (fix it, back to step 3).

Done when the residue is empty or wraps only.

## Finish

Push, `gh pr checks N --watch`, and redo steps 3 and 4 on the new run. Report per check: what was found, what was done, and the numbers.
