# Researching a rule before you write it

Disclosed reference for `add-lint-rule`'s "Before starting" step. Reach for it on two branches: **porting** a rule from another linter, and **grounding** fixtures in real-world code.

## Mining the upstream issue tracker (porting)

Read the original rule's test suite and docs first — cover what they cover, not just the happy path. Then search the tracker for the rule code (e.g. `H019`) and slug (e.g. `javascript-url`), open and closed:

```
gh search issues --repo <owner>/<repo> "<RULE_CODE> OR <rule-slug>" --limit 30 --json number,state,title
```

Read every issue and PR that names the rule. Three payloads, each a fixture:

- **Rejected feature requests** ("also flag X") — encode deliberate "we don't flag X" decisions → a `valid.html` case.
- **Closed false-positive reports** → a `valid.html` case.
- **The original feature request** — its minimal reproducer is usually the canonical true-positive → an `invalid.html` case.

Anchor every issue-derived case with an inline `<!-- Regression: <linter> #<num> — <rationale> -->` comment so it can't be quietly removed later.

## Grounding fixtures in real code (Sourcegraph)

Synthetic cases like `href="http://example.com"` prove the logic but not that the rule fires on code people actually write. Pull real occurrences from [Sourcegraph](https://sourcegraph.com/search) and adapt a representative handful into `invalid.html`.

Query the **streaming search API** through `WebFetch` — the web UI is a JS SPA that returns nothing when fetched, but the API streams plain text:

```
https://sourcegraph.com/.api/search/stream?q=<url-encoded-query>&display=40
```

Build the query from the pattern, scoped to templates — e.g. `src="http://` `lang:HTML` `count:40`. Useful filters: `lang:HTML`, `file:templates/.*\.html$`, `count:N`; append `patternType:regexp` for a regex pattern.

- Pick cases that add **distinct shapes** (a CDN script, a tracking pixel, an external link), not five variants of one. Group under a `<!-- Real-world … -->` comment.
- Results often reveal what to **exempt**: a "violation" that shows up overwhelmingly benign (e.g. `http://localhost:5173` dev servers) belongs in `valid.html`, with the rule taught to skip it.
- Stay faithful — prefer hosts and snippets you actually found over invented ones.
