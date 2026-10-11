---
name: babysit-pr
description: Drive an open djangofmt PR to green — conflicts, failing CI jobs, review threads, CodSpeed and ecosystem-check verdicts.
disable-model-invocation: true
---

# Babysit PR

Takes a PR number; default is the current branch's PR. Fix everything locally, then push **once**: CodSpeed is slow.

1. **Conflicts**: rebase on `main` only when the PR is `CONFLICTING`. Done when it's `MERGEABLE`.
2. **Failing CI jobs**: reproduce each with its `just` recipe. Done when each passes locally.
3. **Review threads** (GraphQL `reviewThreads`): fix them in one commit, or skip the wrong ones. Done when each unresolved thread is fixed or skipped.
4. **CodSpeed**: follow `codspeed:codspeed-optimize`. Done when green, or the regression is acknowledged with its reason in the PR.
5. **Ecosystem check**: read the `Run djangofmt ecosystem check` step of the job log; the PR comment and job summary are truncated. Every removed line must map to an added line through the PR's intended transformation: strip that change from both sides with a script and multiset-compare. Done when the residue is empty or only line wraps from longer output.

## Finish

Push, resolve every step 3 thread with `resolveReviewThread` without replying, and watch the new run. Report per check what was found, what was done, and the numbers, including skipped threads and anything still red.
