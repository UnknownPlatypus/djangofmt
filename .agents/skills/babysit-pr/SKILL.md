---
name: babysit-pr
description: Drive an open djangofmt PR to green — conflicts, failing CI jobs, review threads, CodSpeed and ecosystem-check verdicts.
disable-model-invocation: true
---

# Babysit PR

Takes a PR number; default is the current branch's PR. Work through the five checks, then push **once** so CI (and CodSpeed, which is the slow one) runs a single time.

## 1. Conflicts

`gh pr view N --json mergeable,mergeStateStatus`. `CONFLICTING` → `git rebase origin/main`, resolve, re-run the tests the conflict touched. `MERGEABLE` → leave the branch alone, even when behind `main`.

Done when `mergeable` is `MERGEABLE`.

## 2. Failing CI jobs

`gh pr checks N` lists the failures (CodSpeed and the ecosystem check are steps 4 and 5). Read the log with `gh run view <run-id> --log-failed`, reproduce with the matching `just` recipe, and fix the cause. `just pre-mr-check` runs them all.

Done when every recipe that failed in CI passes locally.

## 3. Review threads

`gh pr view` doesn't expose review threads; use GraphQL:

```bash
gh api graphql -f query='query{repository(owner:"UnknownPlatypus",name:"djangofmt"){pullRequest(number:N){reviewThreads(first:50){nodes{id isResolved path line comments(first:20){nodes{author{login} body}}}}}}}' --jq '.data.repository.pullRequest.reviewThreads.nodes[]|select(.isResolved|not)'
```

Done when each unresolved thread is fixed in one commit or skipped.

## 4. CodSpeed

A failing `CodSpeed Performance Analysis` check: follow `codspeed:codspeed-optimize`.

Done when the check is green, or the regression is acknowledged with its reason in the PR.

## 5. Ecosystem check

Read the full output, never the PR comment, which is truncated. The job log carries it untruncated (the job summary also caps at 1000k); the job id is in the `Ecosystem check` link from `gh pr checks N`:

```bash
gh run view --job <job-id> --log | awk -F'\t' '$2=="Run `djangofmt` ecosystem check"{sub(/^[^ ]+ /,"",$3); print $3}' > ecosystem.md
```

To reproduce locally: `just ecosystem-check <main-binary> target/debug/djangofmt` (`ecosystem-check-lint` for lint changes).

Every removed line must map to an added line through the PR's intended transformation. Classify the whole diff with a normalizing script (strip the intended change from both sides, then multiset-compare) rather than reading files; the residue is either line wraps caused by longer output (fine) or a regression (fix it, back to step 4).

Done when the residue is empty or wraps only.

## Finish

Push, then resolve every thread from step 3 without replying, fixed or skipped:

```bash
gh api graphql -f query='mutation{resolveReviewThread(input:{threadId:"<id>"}){thread{isResolved}}}'
```

`gh pr checks N --watch`, then redo steps 2, 4 and 5 on the new run; anything still red goes in the report for the next run. Report per check: what was found, what was done, and the numbers. List skipped threads with the reason for each.
