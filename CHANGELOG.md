# Changelog

## [1.0.0](https://github.com/UnknownPlatypus/djangofmt/compare/v0.2.12..v1.0.0) - 2026-09-20

[**Docs**](https://unknownplatypus.github.io/djangofmt/docs/) | [**Playground**](https://unknownplatypus.github.io/djangofmt/)

> 🎉 **djangofmt 1.0 is here!**
> The formatter is now stable, hardened by extensive fuzzing, ecosystem check and bug fixing.
> The new `check` command also gained automatic Tailwind CSS class sorting, powered by [rustywind](https://github.com/avencera/rustywind).

### ⛰️ Features

- *(check)* Add `--output-format concise` - ([4fe930f](https://github.com/UnknownPlatypus/djangofmt/commit/4fe930f7286ba058e3c3959c2423e6642926e9c0))
- *(cli)* Add `--target-version` option ([#492](https://github.com/UnknownPlatypus/djangofmt/issues/492)) - ([eb317d3](https://github.com/UnknownPlatypus/djangofmt/commit/eb317d315d772343b19f05a817516c8ebb0b67f2))
- *(cli)* Add `{# djangofmt: file-ignore[...] #}` file-level opt-outs ([#440](https://github.com/UnknownPlatypus/djangofmt/issues/440)) - ([a24f7be](https://github.com/UnknownPlatypus/djangofmt/commit/a24f7bec2376503e4860b590922a085ef17dfefc))
- *(cli)* Hint at the conditional open/close tag limitation on parse errors ([#426](https://github.com/UnknownPlatypus/djangofmt/issues/426)) - ([7c6c67b](https://github.com/UnknownPlatypus/djangofmt/commit/7c6c67b8500fca0820e5960e52c43bbe63dfa0a6))
- *(dev)* Add cargo-fuzz idempotency fuzzing for the formatter ([#435](https://github.com/UnknownPlatypus/djangofmt/issues/435)) - ([2e0e66e](https://github.com/UnknownPlatypus/djangofmt/commit/2e0e66e4cab2c2afa83b723e103980337fe59bd1))
- *(ecosystem)* Report exclusions that are no longer needed ([#401](https://github.com/UnknownPlatypus/djangofmt/issues/401)) - ([0fda5d6](https://github.com/UnknownPlatypus/djangofmt/commit/0fda5d674e3a9d260818a27e7f31819c28152c30))
- *(ecosystem)* Add EvaP, djangoproject.com, healthchecks, babybuddy, InvenTree and netbox targets ([#400](https://github.com/UnknownPlatypus/djangofmt/issues/400)) - ([969a64c](https://github.com/UnknownPlatypus/djangofmt/commit/969a64cfcc29d1689e9a8ed453d9d645c27a218c))
- *(ecosystem-check)* Add 12 Django projects to the corpus ([#463](https://github.com/UnknownPlatypus/djangofmt/issues/463)) - ([788afe8](https://github.com/UnknownPlatypus/djangofmt/commit/788afe8aff5233bd3a7f291e63524a7f8b6c5eaa))
- *(format)* Support `--check` option for checking formatting without making changes ([#496](https://github.com/UnknownPlatypus/djangofmt/issues/496)) - ([4d92de4](https://github.com/UnknownPlatypus/djangofmt/commit/4d92de48869726bebe9c3442f5e342f0e4d29f1a))
- *(format)* Accept whitespace around `:` in the ignore directive ([#449](https://github.com/UnknownPlatypus/djangofmt/issues/449)) - ([4f149a2](https://github.com/UnknownPlatypus/djangofmt/commit/4f149a24111f0da2aa08118c77e5ce9d97ffc0a5))
- *(lint)* Stabilize the linter for 1.0 ([#517](https://github.com/UnknownPlatypus/djangofmt/issues/517)) - ([49e56ed](https://github.com/UnknownPlatypus/djangofmt/commit/49e56ed763ebac3e3645dc7e1474975c632cfddd))
- *(lint)* Add a `pedantic` category and leave it out of the default rule set ([#508](https://github.com/UnknownPlatypus/djangofmt/issues/508)) - ([6a7ead9](https://github.com/UnknownPlatypus/djangofmt/commit/6a7ead9e78ba71f01b3b13498d6d75390fcee0e3))
- *(lint)* Add `unused-ignore-code` lint rule ([#487](https://github.com/UnknownPlatypus/djangofmt/issues/487)) - ([486e2d6](https://github.com/UnknownPlatypus/djangofmt/commit/486e2d6c2db32b06bea1081badce6865c79605c4))
- *(lint)* Add `deprecated-ignore` lint rule ([#443](https://github.com/UnknownPlatypus/djangofmt/issues/443)) - ([59b2d58](https://github.com/UnknownPlatypus/djangofmt/commit/59b2d586b994164cc66adcade4dab72eabb4fa2e))
- *(lint)* Infer `target-version` from the `django` requirement ([#495](https://github.com/UnknownPlatypus/djangofmt/issues/495)) - ([95b98e6](https://github.com/UnknownPlatypus/djangofmt/commit/95b98e667175606e44a7a283300679081929e96a))
- *(lint)* Add `invalid-ignore-comment` and `invalid-ignore-code` lint rules ([#442](https://github.com/UnknownPlatypus/djangofmt/issues/442)) - ([6e8f422](https://github.com/UnknownPlatypus/djangofmt/commit/6e8f422081535065d7933d11446582e787afac74))
- *(lint)* Suppress diagnostics with `{# djangofmt: ignore[rule] #}` comments ([#441](https://github.com/UnknownPlatypus/djangofmt/issues/441)) - ([ef3335f](https://github.com/UnknownPlatypus/djangofmt/commit/ef3335f776df4a274a54d11de59a0a5c40ea5183))
- *(lint)* Honor rule codes in `{# djangofmt: file-ignore[...] #}` ([#470](https://github.com/UnknownPlatypus/djangofmt/issues/470)) - ([a4aea6d](https://github.com/UnknownPlatypus/djangofmt/commit/a4aea6de004f25750ccb2f89899932966a00d992))
- *(lint)* Add `unsorted-tailwind-classes` lint rule ([#332](https://github.com/UnknownPlatypus/djangofmt/issues/332)) - ([da2a7aa](https://github.com/UnknownPlatypus/djangofmt/commit/da2a7aaf233b5e6e4a3290929e833abea43e7ad7))
- *(lint)* Add `same-file-partial-include` lint rule ([#344](https://github.com/UnknownPlatypus/djangofmt/issues/344)) - ([a103419](https://github.com/UnknownPlatypus/djangofmt/commit/a1034193dbdd526c79d7e00d0eea996d03469749))
- *(lint)* Add `per-file-ignores` configuration ([#411](https://github.com/UnknownPlatypus/djangofmt/issues/411)) - ([fefdd9b](https://github.com/UnknownPlatypus/djangofmt/commit/fefdd9b173c0607bf921e51cd118a1c620307275))

### 🐛 Bug Fixes

- *(check)* Render parse errors concisely with `--output-format concise` ([#394](https://github.com/UnknownPlatypus/djangofmt/issues/394)) - ([b821e10](https://github.com/UnknownPlatypus/djangofmt/commit/b821e101826b60a55df2023b74f571c27b041c65))
- *(ci)* Actually truncate the ecosystem comment body - ([58bbd22](https://github.com/UnknownPlatypus/djangofmt/commit/58bbd22bf70473aa9b60d677eaafd5b9f81a6255))
- *(cli)* Stop breaking help URLs across lines ([#458](https://github.com/UnknownPlatypus/djangofmt/issues/458)) - ([63af806](https://github.com/UnknownPlatypus/djangofmt/commit/63af80616eea2b22b85221f402e89c271c21c313))
- *(cli)* Honor a leading `djangofmt:ignore` on unparsable files in `check` ([#439](https://github.com/UnknownPlatypus/djangofmt/issues/439)) - ([8974a3b](https://github.com/UnknownPlatypus/djangofmt/commit/8974a3bf42750202ff39c4c5b899311f0204d64d))
- *(cli)* Anchor include and exclude patterns at the project root ([#433](https://github.com/UnknownPlatypus/djangofmt/issues/433)) - ([2e98311](https://github.com/UnknownPlatypus/djangofmt/commit/2e98311cdd9e3cdf9af417ce9dc8d17ae771abbb))
- *(cli)* Report accurate parse error locations ([#430](https://github.com/UnknownPlatypus/djangofmt/issues/430)) - ([de1ddde](https://github.com/UnknownPlatypus/djangofmt/commit/de1ddde8a176cfc80cb5a5380e01636941453d1b))
- *(cli)* Exit 2 on I/O and parse errors in file mode ([#429](https://github.com/UnknownPlatypus/djangofmt/issues/429)) - ([34f5745](https://github.com/UnknownPlatypus/djangofmt/commit/34f574561d6494124a6359d8c9a35920f540d3c6))
- *(format)* Reject self-closing non-void HTML elements as a parse error ([#516](https://github.com/UnknownPlatypus/djangofmt/issues/516)) - ([2e90033](https://github.com/UnknownPlatypus/djangofmt/commit/2e90033ad2c6a528dc7ced36da817755d1f367b7))
- *(format)* Keep a comment-only CSS block instead of emptying it ([#499](https://github.com/UnknownPlatypus/djangofmt/issues/499)) - ([f39c760](https://github.com/UnknownPlatypus/djangofmt/commit/f39c760bc7ac6c25a3a0effe82cd624ec5de6971))
- *(format)* Keep `/*!`, `/*#` and `/**` CSS comment markers intact ([#498](https://github.com/UnknownPlatypus/djangofmt/issues/498)) - ([25c46f7](https://github.com/UnknownPlatypus/djangofmt/commit/25c46f7fe65487a887fe5f778f70055d32130336))
- *(format)* Leave JSON with raw control characters in strings unformatted ([#485](https://github.com/UnknownPlatypus/djangofmt/issues/485)) - ([69edc82](https://github.com/UnknownPlatypus/djangofmt/commit/69edc82e712422c339ed0e6e364cfc6ed501d850))
- *(format)* Stop reordering CSS shorthands past the longhands they override ([#483](https://github.com/UnknownPlatypus/djangofmt/issues/483)) - ([c77d667](https://github.com/UnknownPlatypus/djangofmt/commit/c77d667ae670fc1f92c7dccfe9d0108d922cedcd))
- *(format)* Respect internal attr loop separator ([#480](https://github.com/UnknownPlatypus/djangofmt/issues/480)) - ([11aee2f](https://github.com/UnknownPlatypus/djangofmt/commit/11aee2ff0d0cdf053393d3111d9b2ada2dcfabd1))
- *(format)* Honor `{# djangofmt: ignore[format] #}` on the next node ([#473](https://github.com/UnknownPlatypus/djangofmt/issues/473)) - ([e4ea817](https://github.com/UnknownPlatypus/djangofmt/commit/e4ea817012a391e1e909c545af917886088d4e64))
- *(format)* Reject malformed template syntax instead of silently corrupting it ([#457](https://github.com/UnknownPlatypus/djangofmt/issues/457)) - ([04e8d72](https://github.com/UnknownPlatypus/djangofmt/commit/04e8d72f6f94a1e520b7861838c52285ddb229eb))
- *(format)* Recover from panics instead of crashing the whole run ([#459](https://github.com/UnknownPlatypus/djangofmt/issues/459)) - ([5c3837b](https://github.com/UnknownPlatypus/djangofmt/commit/5c3837b3d4b0a2fe1f39be043d05b4587ffb2c01))
- *(format)* Keep the opening tag of an ignored nested block tag ([#434](https://github.com/UnknownPlatypus/djangofmt/issues/434)) - ([4a56d53](https://github.com/UnknownPlatypus/djangofmt/commit/4a56d53e80c767fee1dc459b137cd5c49be5e640))
- *(format)* Resolve stdin profile with the same precedence as files ([#413](https://github.com/UnknownPlatypus/djangofmt/issues/413)) - ([78bed0d](https://github.com/UnknownPlatypus/djangofmt/commit/78bed0d497f2af3c56c5d4069d4746fe11f0f9e3))
- *(format)* Don't treat `---` as front matter outside the top of the file ([#407](https://github.com/UnknownPlatypus/djangofmt/issues/407)) - ([d41752d](https://github.com/UnknownPlatypus/djangofmt/commit/d41752d28abba97ac0d09d47e8df52ed965f99cf))
- *(format)* Only treat Django's built-in tags as block tags ([#402](https://github.com/UnknownPlatypus/djangofmt/issues/402)) - ([275acaa](https://github.com/UnknownPlatypus/djangofmt/commit/275acaa2df5d48d600d29cc9161fa86852651e15))
- *(format)* Re-indent `{% endcomment %}` with its block ([#395](https://github.com/UnknownPlatypus/djangofmt/issues/395)) - ([6612b1b](https://github.com/UnknownPlatypus/djangofmt/commit/6612b1be14bfe4adf7b089f2d11df04e8feaa652))
- *(format)* Don't panic on `{% comment %}` among tag attributes ([#393](https://github.com/UnknownPlatypus/djangofmt/issues/393)) - ([9522c6e](https://github.com/UnknownPlatypus/djangofmt/commit/9522c6e34e34a11647ddb2fac98b0382c22b0788))
- *(lint)* More small linter fixes and perf improvements ([#521](https://github.com/UnknownPlatypus/djangofmt/issues/521)) - ([efd6d4d](https://github.com/UnknownPlatypus/djangofmt/commit/efd6d4dddfaeeb511234a3990376635268ff91f7))
- *(lint)* Minor lint rule fixes ([#520](https://github.com/UnknownPlatypus/djangofmt/issues/520)) - ([fc3eec9](https://github.com/UnknownPlatypus/djangofmt/commit/fc3eec906f601d3bb293cc219888a20bdc3ed5c2))
- *(lint)* Detect whitespace-padded URLs in django-static-url and django-url-pattern ([#431](https://github.com/UnknownPlatypus/djangofmt/issues/431)) - ([7de1dd7](https://github.com/UnknownPlatypus/djangofmt/commit/7de1dd7aba1f9c36d5ec29ee217f5a2cb373f2dd))
- *(lint)* Don't disable missing-doctype on root-level Jinja blocks ([#427](https://github.com/UnknownPlatypus/djangofmt/issues/427)) - ([b74d4d4](https://github.com/UnknownPlatypus/djangofmt/commit/b74d4d4300772cd521cf87e6b1aec6504cd6860d))
- *(lint)* Restore build for `form-action-whitespace` fix content ([#399](https://github.com/UnknownPlatypus/djangofmt/issues/399)) - ([1ec0946](https://github.com/UnknownPlatypus/djangofmt/commit/1ec0946d1a38fdcb26138d67c34dd37632d263fa))
- *(playground)* Fix border of plus/minus inputs ([#382](https://github.com/UnknownPlatypus/djangofmt/issues/382)) - ([c026381](https://github.com/UnknownPlatypus/djangofmt/commit/c0263816fe9e38f9b73a46e57b1b238f8a5b0420))
- *(release)* Opt the binary back into `dist` after `publish = false` ([#509](https://github.com/UnknownPlatypus/djangofmt/issues/509)) - ([2735037](https://github.com/UnknownPlatypus/djangofmt/commit/2735037c2ea37ed2493c175f9b186d211a6d35a3))

### 🚜 Refactor

- *(cli)* Count check results in a single pass ([#468](https://github.com/UnknownPlatypus/djangofmt/issues/468)) - ([06e3aa5](https://github.com/UnknownPlatypus/djangofmt/commit/06e3aa59c422dd9564b4f3815dffc5c36578ede6))
- *(cli)* Unify parsing into a `lint_source` and `parse_source` interface ([#404](https://github.com/UnknownPlatypus/djangofmt/issues/404)) - ([017e23e](https://github.com/UnknownPlatypus/djangofmt/commit/017e23ea77df8f9bef117a2d09c2ff3792cdfba7))
- *(dev)* Move generated-page prose out of Rust into Markdown files ([#437](https://github.com/UnknownPlatypus/djangofmt/issues/437)) - ([1781144](https://github.com/UnknownPlatypus/djangofmt/commit/178114403e8a4ecafd1da03b7631fedf00b30cfd))
- *(ecosystem)* Document ecosystem exclusion reasons ([#410](https://github.com/UnknownPlatypus/djangofmt/issues/410)) - ([96f070c](https://github.com/UnknownPlatypus/djangofmt/commit/96f070cd6492632b43f4b615859705e3c9297da9))
- *(format)* Delete dead display only fix api ([#414](https://github.com/UnknownPlatypus/djangofmt/issues/414)) - ([2e5b7d2](https://github.com/UnknownPlatypus/djangofmt/commit/2e5b7d27530e6f3fcfad25c5085ea69f18217bb4))
- *(lint)* Pass checker as the first argument to rule check functions ([#505](https://github.com/UnknownPlatypus/djangofmt/issues/505)) - ([5678cbb](https://github.com/UnknownPlatypus/djangofmt/commit/5678cbb316e0cabf98366f57a7e8979f82885ffc))
- *(lint)* Route `check`, `--fix` and the playground through one `lint_text` api ([#490](https://github.com/UnknownPlatypus/djangofmt/issues/490)) - ([6c8d3e9](https://github.com/UnknownPlatypus/djangofmt/commit/6c8d3e9ea11f3789a636316df744a22371f6dd8e))
- *(lint)* Add `source_span`/`source_end` helpers and derive spans from slices ([#484](https://github.com/UnknownPlatypus/djangofmt/issues/484)) - ([35653ce](https://github.com/UnknownPlatypus/djangofmt/commit/35653ce377ad3e6bfc282832795bb501f3712193))
- *(lint)* Tighten diagnostic messages and help text ([#481](https://github.com/UnknownPlatypus/djangofmt/issues/481)) - ([253e26f](https://github.com/UnknownPlatypus/djangofmt/commit/253e26f48183897b638346d91096e9827defb836))
- *(lint)* Parse suppression directives with one shared grammar ([#472](https://github.com/UnknownPlatypus/djangofmt/issues/472)) - ([e8a5667](https://github.com/UnknownPlatypus/djangofmt/commit/e8a5667ce16ce49e86c1bbac5e58e1a79b45063a))

### 📚 Documentation

- *(agents)* Sync the add-lint-rule skill and AGENTS.md with the current code - ([0226c88](https://github.com/UnknownPlatypus/djangofmt/commit/0226c888f0833929bbe112519d63a2a1d52eccdf))
- *(benchmark)* Update benchmark numbers ([#454](https://github.com/UnknownPlatypus/djangofmt/issues/454)) - ([c725574](https://github.com/UnknownPlatypus/djangofmt/commit/c72557493533d0296125e459188eeda1c62e1282))
- *(format)* Document the omitted end tag limitation and link the hint to it ([#515](https://github.com/UnknownPlatypus/djangofmt/issues/515)) - ([7d00e1a](https://github.com/UnknownPlatypus/djangofmt/commit/7d00e1aa68c8dc8e9d546e6c0a193f7b65790c1a))
- *(skill)* Insist rule doc comments say what, never how ([#424](https://github.com/UnknownPlatypus/djangofmt/issues/424)) - ([2b6a243](https://github.com/UnknownPlatypus/djangofmt/commit/2b6a2431172b87b655505a788f0f52fcb29beae9))
- Document LF-only output as a known limitation ([#461](https://github.com/UnknownPlatypus/djangofmt/issues/461)) - ([de59f2b](https://github.com/UnknownPlatypus/djangofmt/commit/de59f2bc624f2977e44a53c600a6e194098e0664))

### ⚡ Performance

- *(bench)* Add a reporter benchmark covering diagnostic rendering ([#390](https://github.com/UnknownPlatypus/djangofmt/issues/390)) - ([70c2820](https://github.com/UnknownPlatypus/djangofmt/commit/70c2820ae5ebb85f31c4e81d73e3b0fc0a84a547))
- *(format)* Use `oxc-miette` for ~5x faster diagnostic rendering ([#392](https://github.com/UnknownPlatypus/djangofmt/issues/392)) - ([822a21f](https://github.com/UnknownPlatypus/djangofmt/commit/822a21fb6f87fc8a870e7ef389ace7cf29ce5378))
- *(lint)* Avoid allocating for static violation messages, help and fix titles ([#397](https://github.com/UnknownPlatypus/djangofmt/issues/397)) - ([e67ab92](https://github.com/UnknownPlatypus/djangofmt/commit/e67ab923fed9de61da461969d34af2ad543d5d35))
- *(lint)* Avoid allocating for static fix content ([#398](https://github.com/UnknownPlatypus/djangofmt/issues/398)) - ([72551cc](https://github.com/UnknownPlatypus/djangofmt/commit/72551ccab5d07769d9493d592afae425deabd46f))
- *(lint)* Use &'static str for diagnostic rule codes ([#396](https://github.com/UnknownPlatypus/djangofmt/issues/396)) - ([be7ed4f](https://github.com/UnknownPlatypus/djangofmt/commit/be7ed4f5f5e967bb198663674c07f86b6e86a72c))

### 🧪 Testing

- *(bench)* Benchmark the default rule selection, not just none and all ([#412](https://github.com/UnknownPlatypus/djangofmt/issues/412)) - ([b9e9591](https://github.com/UnknownPlatypus/djangofmt/commit/b9e959101702d406d67337e25e1e9de62602601e))
- *(benchmark)* Benchmark templates carrying a `djangofmt:` directive ([#469](https://github.com/UnknownPlatypus/djangofmt/issues/469)) - ([00c4553](https://github.com/UnknownPlatypus/djangofmt/commit/00c455344ca8a1888efb7a0ed3b7aebeab152e26))
- *(ecosystem)* Drop the stale bookwyrm `manual-merge.html` exclusion ([#497](https://github.com/UnknownPlatypus/djangofmt/issues/497)) - ([9be0f74](https://github.com/UnknownPlatypus/djangofmt/commit/9be0f74df6aa2fe4c31ab1613471a04d6737cffb))
- *(ecosystem)* Exclude django-unfold `avatar.html` (dynamic tag name) ([#455](https://github.com/UnknownPlatypus/djangofmt/issues/455)) - ([30ba2c8](https://github.com/UnknownPlatypus/djangofmt/commit/30ba2c8ace0dc87689bf8446268a81b22201cdfd))
- *(ecosystem)* Update ecosystem exclusion list and also check stability false positives ([#403](https://github.com/UnknownPlatypus/djangofmt/issues/403)) - ([671704f](https://github.com/UnknownPlatypus/djangofmt/commit/671704f130a45215f2d093a045422b1ad24f0b38))
- *(ecosystem-check)* Report fixable lint changes as a source diff - ([cfa4d66](https://github.com/UnknownPlatypus/djangofmt/commit/cfa4d669650f070cc2651753431ef9719e040598))
- *(format)* Pin lowercased CSS property names as intended ([#486](https://github.com/UnknownPlatypus/djangofmt/issues/486)) - ([60d599d](https://github.com/UnknownPlatypus/djangofmt/commit/60d599d1f463e1a03ae31fee8c6093c57d974703))
- *(fuzz)* Skip control-character inputs in the idempotency target ([#504](https://github.com/UnknownPlatypus/djangofmt/issues/504)) - ([1f0b3df](https://github.com/UnknownPlatypus/djangofmt/commit/1f0b3df0aa943a8e3e988944ef1f16299b6f42f3))
- *(refactor)* Drop tests already covered elsewhere ([#438](https://github.com/UnknownPlatypus/djangofmt/issues/438)) - ([e5b827f](https://github.com/UnknownPlatypus/djangofmt/commit/e5b827f4fa3bd6546dcb65ad803880c5c91402fc))
- *(refactor)* Each rule test fixture now only enable the rule ([#406](https://github.com/UnknownPlatypus/djangofmt/issues/406)) - ([d0921df](https://github.com/UnknownPlatypus/djangofmt/commit/d0921dfbb1c96f11bba66a227887b4d1f5d034ea))

### ⚙️ Miscellaneous Tasks

- *(benchmark)* Fix codspeed flakiness after v5 ([#448](https://github.com/UnknownPlatypus/djangofmt/issues/448)) - ([2dfa651](https://github.com/UnknownPlatypus/djangofmt/commit/2dfa6519e72f81018dfd6bf9379dd03b4709ec66))
- *(changelog)* Add a one-off 1.0 banner to the generated changelog ([#464](https://github.com/UnknownPlatypus/djangofmt/issues/464)) - ([d4994ef](https://github.com/UnknownPlatypus/djangofmt/commit/d4994effbb57632d4362a74ee7bc016f67837e3f))
- *(ecosystem)* Add a parse check that fails when formatting breaks templates ([#494](https://github.com/UnknownPlatypus/djangofmt/issues/494)) - ([43a6507](https://github.com/UnknownPlatypus/djangofmt/commit/43a650712bbc646caa90d499ff0b8ef7274ca0ef))
- *(ecosystem)* Stop rebuilding `djangofmt` a third time in the ecosystem check ([#456](https://github.com/UnknownPlatypus/djangofmt/issues/456)) - ([43fd237](https://github.com/UnknownPlatypus/djangofmt/commit/43fd2378544557d05d88d5b24e7ed1882c9a634e))
- *(release)* Fix s390x cross build of psm assembly ([#453](https://github.com/UnknownPlatypus/djangofmt/issues/453)) - ([0d8e8de](https://github.com/UnknownPlatypus/djangofmt/commit/0d8e8de31c90a3519ff2ea70d2c93ec3efda2b31))
- *(security)* Reject a tampered `pr-number` artifact before it reaches `GITHUB_OUTPUT` ([#514](https://github.com/UnknownPlatypus/djangofmt/issues/514)) - ([21a11e0](https://github.com/UnknownPlatypus/djangofmt/commit/21a11e0fbdb4e136929f7c78549f978d1b1ee962))
- *(security)* Harden release workflow permissions, inputs, and caching ([#422](https://github.com/UnknownPlatypus/djangofmt/issues/422)) - ([4df167d](https://github.com/UnknownPlatypus/djangofmt/commit/4df167d0b309b66c1ede09a164f697809fd2d07a))
- *(stability)* Reduce codspeed variance ([#465](https://github.com/UnknownPlatypus/djangofmt/issues/465)) - ([657918f](https://github.com/UnknownPlatypus/djangofmt/commit/657918fcd069280169140b61eff9827c8ba52e01))
- *(tests)* Regenerate snapshots in insta's current format ([#475](https://github.com/UnknownPlatypus/djangofmt/issues/475)) - ([23cca63](https://github.com/UnknownPlatypus/djangofmt/commit/23cca6383314e7dcfb83593268fdbe1420c85b24))
- Pin cargo dev tools through uv.lock ([#477](https://github.com/UnknownPlatypus/djangofmt/issues/477)) - ([f92f89c](https://github.com/UnknownPlatypus/djangofmt/commit/f92f89c07a3e20657c08865d805f265148774525))
- Drop the cargo-bloat workflow for a just recipe ([#479](https://github.com/UnknownPlatypus/djangofmt/issues/479)) - ([110c8a2](https://github.com/UnknownPlatypus/djangofmt/commit/110c8a24cd09277148a0b0fef0cc32577846c660))
- Build and test with --locked ([#476](https://github.com/UnknownPlatypus/djangofmt/issues/476)) - ([7a87cea](https://github.com/UnknownPlatypus/djangofmt/commit/7a87cea44dfdbf83fb3f5830a55ed23b80c0a3ca))
- Set UV_LOCKED in workflows that resolve dependencies ([#478](https://github.com/UnknownPlatypus/djangofmt/issues/478)) - ([81e72fd](https://github.com/UnknownPlatypus/djangofmt/commit/81e72fd3d40d543b76bb434b03a07a63fbf176e4))

### New Contributors ❤️

- @nzlaura made their first contribution in [#496](https://github.com/UnknownPlatypus/djangofmt/pull/496)

## [0.2.12](https://github.com/UnknownPlatypus/djangofmt/compare/v0.2.11..v0.2.12) - 2026-07-15

[**Docs**](https://unknownplatypus.github.io/djangofmt/docs/) | [**Playground**](https://unknownplatypus.github.io/djangofmt/)

### ⛰️ Features

- *(lint)* Add `table-header-missing-scope` lint rule ([#345](https://github.com/UnknownPlatypus/djangofmt/issues/345)) - ([4d9f72c](https://github.com/UnknownPlatypus/djangofmt/commit/4d9f72c7bf953bc0d69366243fb213491c67bd32))

### 🐛 Bug Fixes

- *(format)* Skip formatting Django `verbatim` blocks ([#370](https://github.com/UnknownPlatypus/djangofmt/issues/370)) - ([caf2fe4](https://github.com/UnknownPlatypus/djangofmt/commit/caf2fe41c3f4c1b35d64682973583daa3b2b7aa2))

## [0.2.11](https://github.com/UnknownPlatypus/djangofmt/compare/v0.2.10..v0.2.11) - 2026-06-26

[**Docs**](https://unknownplatypus.github.io/djangofmt/docs/) | [**Playground**](https://unknownplatypus.github.io/djangofmt/)

### ⛰️ Features

- *(format)* Preserve single-line Django blocks inside attribute lists ([#360](https://github.com/UnknownPlatypus/djangofmt/issues/360)) - ([ca3d825](https://github.com/UnknownPlatypus/djangofmt/commit/ca3d825aa7dd8f1eff93d76bd52670085c3b3682))
- *(format)* Support `.editorconfig` configuration files ([#349](https://github.com/UnknownPlatypus/djangofmt/issues/349)) - ([0df2c71](https://github.com/UnknownPlatypus/djangofmt/commit/0df2c71c63db5c497cbf4d6e34cd3bb42d644f58))
- *(lint)* Add duplicate-block-name lint rule - ([4fe9aef](https://github.com/UnknownPlatypus/djangofmt/commit/4fe9aef5534b55680b1e921a99cc9bb329dd4a10))
- *(lint)* Add rule selection (select/ignore + preview) ([#350](https://github.com/UnknownPlatypus/djangofmt/issues/350)) - ([7d0eb76](https://github.com/UnknownPlatypus/djangofmt/commit/7d0eb76d78eca7d94fa420d522dc42e46ce97646))
- *(playground)* Add documentation link to navbar ([#351](https://github.com/UnknownPlatypus/djangofmt/issues/351)) - ([0923257](https://github.com/UnknownPlatypus/djangofmt/commit/0923257c860fa39a50c2c1ccdf6e02cb7964e8b4))

### 🐛 Bug Fixes

- *(format)* Fix formatting of multiline django template tags ([#359](https://github.com/UnknownPlatypus/djangofmt/issues/359)) - ([2908461](https://github.com/UnknownPlatypus/djangofmt/commit/290846188d5a3c2920f8d29edf81f944abe328b5))
- *(lint)* Restore correct ANSI display in error messages ([#346](https://github.com/UnknownPlatypus/djangofmt/issues/346)) - ([d4c21ba](https://github.com/UnknownPlatypus/djangofmt/commit/d4c21ba49ed3de0f7d5212ccd7f645512789b8ec))

### 🚜 Refactor

- *(lint)* Avoid double parse when fixing ([#362](https://github.com/UnknownPlatypus/djangofmt/issues/362)) - ([621980e](https://github.com/UnknownPlatypus/djangofmt/commit/621980e9988a896e094903a9705d60c8fe1fb2ae))
- *(lint)* Drop redundant help on empty-attr-value rule ([#357](https://github.com/UnknownPlatypus/djangofmt/issues/357)) - ([e10ed1e](https://github.com/UnknownPlatypus/djangofmt/commit/e10ed1e91783d1db5ead6d56acd687e7270497ca))
- *(lint)* Cleanup RuleSet implementation and simplify ([#336](https://github.com/UnknownPlatypus/djangofmt/issues/336)) - ([17c0668](https://github.com/UnknownPlatypus/djangofmt/commit/17c06682b8dda632b0d343b60a79cc6e2ca9b5b8))
- *(misc)* Minor cleanup ([#356](https://github.com/UnknownPlatypus/djangofmt/issues/356)) - ([2b1346b](https://github.com/UnknownPlatypus/djangofmt/commit/2b1346bdb144bfed8c0fc3020a9cbba1d4fa69f7))
- *(tests)* Misc test simplification ([#348](https://github.com/UnknownPlatypus/djangofmt/issues/348)) - ([cdda76a](https://github.com/UnknownPlatypus/djangofmt/commit/cdda76a57c2ed99c50bbfc6dcc7d93a74179f5d7))

### 🧪 Testing

- *(bench)* Isolate check_ast cost from parsing ([#364](https://github.com/UnknownPlatypus/djangofmt/issues/364)) - ([8251c11](https://github.com/UnknownPlatypus/djangofmt/commit/8251c110d182c192b71e67dee9821c621fc9183d))
- *(clippy)* Enable more clippy restriction rules ([#363](https://github.com/UnknownPlatypus/djangofmt/issues/363)) - ([539450c](https://github.com/UnknownPlatypus/djangofmt/commit/539450c56345ee74bf482a6cc5953f243c4621e3))
- *(ecosystem-check)* Select all rules + preview in check ecosystem run ([#352](https://github.com/UnknownPlatypus/djangofmt/issues/352)) - ([17110ef](https://github.com/UnknownPlatypus/djangofmt/commit/17110efb4e931ea97ba9320db6a88e7c59f379f9))

### ⚙️ Miscellaneous Tasks

- *(codspeed)* Pin glibc CPU-feature dispatch to reduce benchmark variance ([#366](https://github.com/UnknownPlatypus/djangofmt/issues/366)) - ([5175539](https://github.com/UnknownPlatypus/djangofmt/commit/5175539b81f8836c533a66748d117d8d0b42667b))

## [0.2.10](https://github.com/UnknownPlatypus/djangofmt/compare/v0.2.9..v0.2.10) - 2026-06-03

[**Docs**](https://unknownplatypus.github.io/djangofmt/docs/) | [**Playground**](https://unknownplatypus.github.io/djangofmt/)

### ⛰️ Features

- *(format)* Allow `{# djangofmt:ignore #}` to disable formatting ([#333](https://github.com/UnknownPlatypus/djangofmt/issues/333)) - ([8de13fc](https://github.com/UnknownPlatypus/djangofmt/commit/8de13fc09e22779d903dda472f506b44b827954e))
- *(lint)* Add `django-url-pattern` lint rule ([#314](https://github.com/UnknownPlatypus/djangofmt/issues/314)) - ([9cfc71c](https://github.com/UnknownPlatypus/djangofmt/commit/9cfc71c9fc530f18f04bbcf49d622aecbf9ee4b2))
- *(lint)* Add `empty-tag-pair` lint rule ([#313](https://github.com/UnknownPlatypus/djangofmt/issues/313)) - ([5ac93df](https://github.com/UnknownPlatypus/djangofmt/commit/5ac93dfe72040647eb1568ba7a3e5a1ae68c8e25))
- *(lint)* Add `missing-img-alt` lint rule ([#312](https://github.com/UnknownPlatypus/djangofmt/issues/312)) - ([d7e3d38](https://github.com/UnknownPlatypus/djangofmt/commit/d7e3d3802992d472279cc260fc08761a9fddf29f))
- *(lint)* Add `django-static-url` lint rule ([#311](https://github.com/UnknownPlatypus/djangofmt/issues/311)) - ([27836b6](https://github.com/UnknownPlatypus/djangofmt/commit/27836b62f6cb1236b0168204d9d89ce21412ffb2))
- *(lint)* Add `use-https` lint rule ([#308](https://github.com/UnknownPlatypus/djangofmt/issues/308)) - ([3503bc7](https://github.com/UnknownPlatypus/djangofmt/commit/3503bc73b7fbe9a21ea111456d416be70dd74873))
- *(playground)* Add ast + Doc IR representation in playground ([#331](https://github.com/UnknownPlatypus/djangofmt/issues/331)) - ([c9489bb](https://github.com/UnknownPlatypus/djangofmt/commit/c9489bb8e1291c048d961e7e548100e2d5b38e97))

### 🐛 Bug Fixes

- *(format)* Stabilize indentation of `<script>` tag content ([#335](https://github.com/UnknownPlatypus/djangofmt/issues/335)) - ([8c98e1e](https://github.com/UnknownPlatypus/djangofmt/commit/8c98e1e235bac6cbfe208e2a02c0f2b287f1d83a))

### 🚜 Refactor

- *(lint)* Dispatch per-attribute rules from a single attribute visitor ([#330](https://github.com/UnknownPlatypus/djangofmt/issues/330)) - ([d02925c](https://github.com/UnknownPlatypus/djangofmt/commit/d02925cf815622d92a13b0db2192bdc71c294fd3))
- *(lint)* Dispatch tag-scoped rules by tag ([#326](https://github.com/UnknownPlatypus/djangofmt/issues/326)) - ([4c1a059](https://github.com/UnknownPlatypus/djangofmt/commit/4c1a0594f1d284c21bf4e5bc8b9bbc3bf1e598de))
- *(lint)* Check img height and width attributes in a single pass ([#322](https://github.com/UnknownPlatypus/djangofmt/issues/322)) - ([5164cea](https://github.com/UnknownPlatypus/djangofmt/commit/5164cea1512d440fc563fbcecb11070157004894))
- *(lint)* Reuse fs::get_cwd and drop build_walk_filters wrapper ([#320](https://github.com/UnknownPlatypus/djangofmt/issues/320)) - ([9aab7b7](https://github.com/UnknownPlatypus/djangofmt/commit/9aab7b707538e1c700ce2b02edf6683c09404bc6))
- *(lint)* Add `fix::edits` module with `delete_attr_fix` helper ([#319](https://github.com/UnknownPlatypus/djangofmt/issues/319)) - ([d7cc9ad](https://github.com/UnknownPlatypus/djangofmt/commit/d7cc9ad8b33ccc67a4c56413d14a2e21902fa878))
- *(lint)* Extract jinja-aware attr-presence helper ([#318](https://github.com/UnknownPlatypus/djangofmt/issues/318)) - ([6910bd9](https://github.com/UnknownPlatypus/djangofmt/commit/6910bd972adad609506edd9e82b014fb01071fc9))
- *(lint)* Fold attribute value match into NativeAttribute pattern ([#317](https://github.com/UnknownPlatypus/djangofmt/issues/317)) - ([67f5802](https://github.com/UnknownPlatypus/djangofmt/commit/67f58023fa7a58952cd580996f8d7b1f83d84aba))
- *(playground)* Switch to deno ([#316](https://github.com/UnknownPlatypus/djangofmt/issues/316)) - ([2abd19d](https://github.com/UnknownPlatypus/djangofmt/commit/2abd19d6ff5967ae8fd2e6f9e4a42c5f6194119c))
- *(test)* Share benchmark template list via `ALL_TEMPLATES` const ([#321](https://github.com/UnknownPlatypus/djangofmt/issues/321)) - ([2eab77b](https://github.com/UnknownPlatypus/djangofmt/commit/2eab77b5cb9bf5bdcd4d6a802238fc27419021cc))

### ⚡ Performance

- *(format)* Update `markup_fmt` leading to average perf improvement of 2x ([#329](https://github.com/UnknownPlatypus/djangofmt/issues/329)) - ([d0329ca](https://github.com/UnknownPlatypus/djangofmt/commit/d0329caa34e3fd6d128f1cb5a0f383dfc793d3fc))
- *(lint)* Use a BitSet to store enabled rules ([#325](https://github.com/UnknownPlatypus/djangofmt/issues/325)) - ([347d0b9](https://github.com/UnknownPlatypus/djangofmt/commit/347d0b95f736ca87bd5630a79a5e4f0585f2137b))

## [0.2.9](https://github.com/UnknownPlatypus/djangofmt/compare/v0.2.8..v0.2.9) - 2026-05-29

[**Docs**](https://unknownplatypus.github.io/djangofmt/docs/) | [**Playground**](https://unknownplatypus.github.io/djangofmt/)

### ⛰️ Features

- *(check)* Expose --fix, --unsafe-fixes, --show-fixes in pyproject config ([#290](https://github.com/UnknownPlatypus/djangofmt/issues/290)) - ([448f487](https://github.com/UnknownPlatypus/djangofmt/commit/448f487b01479666f2aee31356e6f111f3f80af0))
- *(docs)* Publish Zensical-based documentation site ([#300](https://github.com/UnknownPlatypus/djangofmt/issues/300)) - ([5f29df3](https://github.com/UnknownPlatypus/djangofmt/commit/5f29df37dab0f641f0aaafc43608ab526b79a294))
- *(lint)* Add `missing-doctype` lint rule ([#310](https://github.com/UnknownPlatypus/djangofmt/issues/310)) - ([e372374](https://github.com/UnknownPlatypus/djangofmt/commit/e3723748b443d8d00234872dce252265fb3d5212))
- *(lint)* Add `missing-title` lint rule ([#309](https://github.com/UnknownPlatypus/djangofmt/issues/309)) - ([b243792](https://github.com/UnknownPlatypus/djangofmt/commit/b243792eb5320978668606c763add93648a258b5))
- *(lint)* Add `duplicate-attr` lint rule ([#304](https://github.com/UnknownPlatypus/djangofmt/issues/304)) - ([ef732af](https://github.com/UnknownPlatypus/djangofmt/commit/ef732afa5cdec1fded8198a34c5928fe3e9876ef))
- *(lint)* Add `form-action-whitespace` lint rule ([#303](https://github.com/UnknownPlatypus/djangofmt/issues/303)) - ([c2b3d53](https://github.com/UnknownPlatypus/djangofmt/commit/c2b3d53beb6fb2cc4ac511c9e2ed4be4eb3c84dd))
- *(lint)* Add `empty-attr-value` lint rule ([#298](https://github.com/UnknownPlatypus/djangofmt/issues/298)) - ([559313c](https://github.com/UnknownPlatypus/djangofmt/commit/559313c473efc0f7e3f54748e8ea26ed40bf9a49))
- *(lint)* Add `uppercase-form-method` lint rule ([#294](https://github.com/UnknownPlatypus/djangofmt/issues/294)) - ([4d37afe](https://github.com/UnknownPlatypus/djangofmt/commit/4d37afe760426e7866b7c9ee355c405fc19bd8e0))
- *(lint)* Add `javascript-url` lint rule ([#293](https://github.com/UnknownPlatypus/djangofmt/issues/293)) - ([791e928](https://github.com/UnknownPlatypus/djangofmt/commit/791e9282c68ab39c7fda12b2fc6ee52e56f00958))
- *(lint)* Add `redundant-type-attr` rule ([#260](https://github.com/UnknownPlatypus/djangofmt/issues/260)) - ([3d65c26](https://github.com/UnknownPlatypus/djangofmt/commit/3d65c267922eff9a2dfacd1d10d6100e5fe3a859))

### 🚜 Refactor

- *(lint)* Align violation structs with owned-data convention ([#296](https://github.com/UnknownPlatypus/djangofmt/issues/296)) - ([1f0e05f](https://github.com/UnknownPlatypus/djangofmt/commit/1f0e05f81a448aa8d9c7c8764701146d36634103))

### 📚 Documentation

- *(lint)* Auto-generate lint rule docs from violation struct doc comments ([#297](https://github.com/UnknownPlatypus/djangofmt/issues/297)) - ([3670b71](https://github.com/UnknownPlatypus/djangofmt/commit/3670b7156eb102799aaa3332a08b0985925fa9ec))
- *(pycharm)* Pycharm editor integration ([#301](https://github.com/UnknownPlatypus/djangofmt/issues/301)) - ([bbf6823](https://github.com/UnknownPlatypus/djangofmt/commit/bbf6823898d656c0169e81dfee6cde62488d37ab))
- *(readme)* Extract some part of the README to dedicated doc page ([#291](https://github.com/UnknownPlatypus/djangofmt/issues/291)) - ([1ea7bae](https://github.com/UnknownPlatypus/djangofmt/commit/1ea7baea683778643547cde205d3244df0ab4654))

### ⚙️ Miscellaneous Tasks

- *(pre-commit)* Chore/pre commit autoupdate and zizmor ([#299](https://github.com/UnknownPlatypus/djangofmt/issues/299)) - ([2dd2c0e](https://github.com/UnknownPlatypus/djangofmt/commit/2dd2c0e0d47ea7566e05e5c1f8575a2f333873e8))
- Scope pages concurrency per PR ([#315](https://github.com/UnknownPlatypus/djangofmt/issues/315)) - ([57ae947](https://github.com/UnknownPlatypus/djangofmt/commit/57ae9476617c9ac8614542ebfe587979208b9935))

## [0.2.8](https://github.com/UnknownPlatypus/djangofmt/compare/v0.2.7..v0.2.8) - 2026-05-18

[**Docs**](https://unknownplatypus.github.io/djangofmt/docs/) | [**Playground**](https://unknownplatypus.github.io/djangofmt/)

### ⛰️ Features

- *(check)* Add autofix framework to the `djangofmt check` command and the `blocktranslate-no-trimmed` rule ([#276](https://github.com/UnknownPlatypus/djangofmt/issues/276)) - ([0b04439](https://github.com/UnknownPlatypus/djangofmt/commit/0b0443991042464e3de9cc7b9a1e75a81c5592a7))
- *(cli)* Format from stdin via `-` or `--stdin-filename` ([#284](https://github.com/UnknownPlatypus/djangofmt/issues/284)) - ([a876757](https://github.com/UnknownPlatypus/djangofmt/commit/a87675721f29e793bc384f306af421bebb76e8ae))
- *(format)* Add `--preserve-unquoted-attrs` option ([#271](https://github.com/UnknownPlatypus/djangofmt/issues/271)) - ([081ab4a](https://github.com/UnknownPlatypus/djangofmt/commit/081ab4a3a70af275e3b38ba0237ced910636ca30))
- *(format)* Add support for Django template partials ([#275](https://github.com/UnknownPlatypus/djangofmt/issues/275)) - ([a13115e](https://github.com/UnknownPlatypus/djangofmt/commit/a13115e7ef3f856eb8540e20d3986b6f5b54583e))

### 🐛 Bug Fixes

- *(docs)* Remove smart quotes from example code ([#267](https://github.com/UnknownPlatypus/djangofmt/issues/267)) - ([adc40d6](https://github.com/UnknownPlatypus/djangofmt/commit/adc40d668a64b406f6bf13ff6141389565399d44))

### ⚙️ Miscellaneous Tasks

- *(msrv)* Track MSRV and rust-toolchain version with renovate ([#281](https://github.com/UnknownPlatypus/djangofmt/issues/281)) - ([690170f](https://github.com/UnknownPlatypus/djangofmt/commit/690170faf4927b0ebe3349d2e9c56f7ad8b8a2ce))
- *(perf)* Include all rust-related files in changed_files detection ([#280](https://github.com/UnknownPlatypus/djangofmt/issues/280)) - ([f182cfa](https://github.com/UnknownPlatypus/djangofmt/commit/f182cfade7ff0fc02e8f64548e033b52d5e11366))
- *(renovate)* Add 7-day cooldown to mitigate supply chain attacks ([#266](https://github.com/UnknownPlatypus/djangofmt/issues/266)) - ([ce3759f](https://github.com/UnknownPlatypus/djangofmt/commit/ce3759f565a80012c13a7a1011406014cd5812fa))
- Set CARGO_PROFILE_DEV_DEBUG=line-tables-only for faster builds ([#270](https://github.com/UnknownPlatypus/djangofmt/issues/270)) - ([06486be](https://github.com/UnknownPlatypus/djangofmt/commit/06486be3ffba4aa55c4f3db966e1f959f48aa72c))

### Build

- *(playground)* Migrate from npm to bun, gate dep release age ([#279](https://github.com/UnknownPlatypus/djangofmt/issues/279)) - ([b4915de](https://github.com/UnknownPlatypus/djangofmt/commit/b4915def7dfa4687c23ae03ed639b3fd0855b4b9))

### New Contributors ❤️

- @nickpetrovic made their first contribution in [#271](https://github.com/UnknownPlatypus/djangofmt/pull/271)
- @meshy made their first contribution in [#267](https://github.com/UnknownPlatypus/djangofmt/pull/267)

## [0.2.7](https://github.com/UnknownPlatypus/djangofmt/compare/v0.2.6..v0.2.7) - 2026-04-16

[**Docs**](https://unknownplatypus.github.io/djangofmt/docs/) | [**Playground**](https://unknownplatypus.github.io/djangofmt/)

### ⛰️ Features

- *(ci)* Build pipeline improvements ([#225](https://github.com/UnknownPlatypus/djangofmt/issues/225)) - ([3e3b009](https://github.com/UnknownPlatypus/djangofmt/commit/3e3b009f89ed039c376409d13aca3fd1faeba879))
- *(config)* Use kebab-case for pyproject.toml settings ([#227](https://github.com/UnknownPlatypus/djangofmt/issues/227)) - ([f26e834](https://github.com/UnknownPlatypus/djangofmt/commit/f26e834b9402f5b90d804ab98b17170645747c18))
- *(format)* Add recursive file discovery -- enable `djangofmt .` to format the current folder ([#229](https://github.com/UnknownPlatypus/djangofmt/issues/229)) - ([385f60a](https://github.com/UnknownPlatypus/djangofmt/commit/385f60a68328a97888e9419ecc2372f47c3becff))
- *(format)* Add flag to allow self closing void elements ([#219](https://github.com/UnknownPlatypus/djangofmt/issues/219)) - ([d0d85b0](https://github.com/UnknownPlatypus/djangofmt/commit/d0d85b0c12637812eb3e65dc6c28ecc1c6b7397a))
- *(format)* Improve error message for cut off html ([#207](https://github.com/UnknownPlatypus/djangofmt/issues/207)) - ([decbaec](https://github.com/UnknownPlatypus/djangofmt/commit/decbaec5460a0a0c0ca47e79b361c8b5e2ebf5e8))
- *(format)* Add JSON external formatter ([#188](https://github.com/UnknownPlatypus/djangofmt/issues/188)) - ([07e770b](https://github.com/UnknownPlatypus/djangofmt/commit/07e770ba4fd37c9e60a52fd49e524cf6267eca4e))

### 🐛 Bug Fixes

- *(format)* Fixes extra indent for {% plural %} inside blocktranslate ([#208](https://github.com/UnknownPlatypus/djangofmt/issues/208)) - ([c7de06d](https://github.com/UnknownPlatypus/djangofmt/commit/c7de06d0f536bc7b99da9760f5e46747f958fd65))
- *(format)* Fix formatting issues for nested template blocks in html opening tag ([#205](https://github.com/UnknownPlatypus/djangofmt/issues/205)) - ([378bc36](https://github.com/UnknownPlatypus/djangofmt/commit/378bc360e18b96f29ae9079aca34c3c98a2a4e04))

### 🚜 Refactor

- *(format)* Reuse `LineLength` and `PrintWidth` newtypes to remove hardcoded default values ([#189](https://github.com/UnknownPlatypus/djangofmt/issues/189)) - ([13e89b5](https://github.com/UnknownPlatypus/djangofmt/commit/13e89b5cd0a8db911ae1d05ae6c0afc272f65498))
- *(misc)* Avoid clone on check error path, add force_exclude tests ([#239](https://github.com/UnknownPlatypus/djangofmt/issues/239)) - ([abc267d](https://github.com/UnknownPlatypus/djangofmt/commit/abc267d1c52ad49c0d47a114b601f259b3d78765))
- *(misc)* Hoist Settings::default(), remove dead Hash derive, avoid clone on error path ([#237](https://github.com/UnknownPlatypus/djangofmt/issues/237)) - ([2923267](https://github.com/UnknownPlatypus/djangofmt/commit/292326797545a0c47325cb1a77201d6f243a2aaf))
- *(misc)* Minor code quality improvement and simplifications ([#235](https://github.com/UnknownPlatypus/djangofmt/issues/235)) - ([1c1c856](https://github.com/UnknownPlatypus/djangofmt/commit/1c1c856033c2dc5bad4daca8e7042589015e999c))
- *(rules)* Declare lint rule category on the Violation ([#230](https://github.com/UnknownPlatypus/djangofmt/issues/230)) - ([6dd81ab](https://github.com/UnknownPlatypus/djangofmt/commit/6dd81ab0c19ef8d11c7d5e507efdf13652fc5dfc))

### 📚 Documentation

- *(ai)* Add AI policy to CONTRIBUTING.md ([#228](https://github.com/UnknownPlatypus/djangofmt/issues/228)) - ([479c575](https://github.com/UnknownPlatypus/djangofmt/commit/479c575646d742641a7ed550059e7d860f649df3))
- *(check)* Document check mode workaround ([#200](https://github.com/UnknownPlatypus/djangofmt/issues/200)) - ([8be5ca6](https://github.com/UnknownPlatypus/djangofmt/commit/8be5ca6025402da808f6ac8de335575854b37ee4))

### ⚡ Performance

- *(bench)* Bench linter and parser ([#232](https://github.com/UnknownPlatypus/djangofmt/issues/232)) - ([d1644eb](https://github.com/UnknownPlatypus/djangofmt/commit/d1644eb60b87f95363e7badf95ccb2d6862adf09))
- *(summary)* Add micro bench build summary ([#236](https://github.com/UnknownPlatypus/djangofmt/issues/236)) - ([6ba6a80](https://github.com/UnknownPlatypus/djangofmt/commit/6ba6a80e7cd7bdba7209d171f843316603f1fb85))

### 🧪 Testing

- *(ci)* Add `djangofmt check` to ecosystem check ([#253](https://github.com/UnknownPlatypus/djangofmt/issues/253)) - ([e9ac547](https://github.com/UnknownPlatypus/djangofmt/commit/e9ac5473fba7b27a5000574f60cb653d69940c24))
- *(ci)* Add code coverage with cargo-llvm-cov and 85% CI threshold ([#240](https://github.com/UnknownPlatypus/djangofmt/issues/240)) - ([78a97a6](https://github.com/UnknownPlatypus/djangofmt/commit/78a97a6a73fde7d8c49dd03e1366ddd5864878b3))
- *(ci)* Re-enable djade ecosystem check (+ tidy up justfile) ([#206](https://github.com/UnknownPlatypus/djangofmt/issues/206)) - ([461db3b](https://github.com/UnknownPlatypus/djangofmt/commit/461db3b9e64d9052eb30af5662d558ae9e1aeae3))
- *(ci)* Remove docker-run-action ([#195](https://github.com/UnknownPlatypus/djangofmt/issues/195)) - ([c1c080b](https://github.com/UnknownPlatypus/djangofmt/commit/c1c080b1ac45f6120c63e8d7d8aa351fa5ad0f75))
- *(ecosystem-check)* Allows pr comment on forks ([#196](https://github.com/UnknownPlatypus/djangofmt/issues/196)) - ([54b204d](https://github.com/UnknownPlatypus/djangofmt/commit/54b204d9a080cb2e7095adab6d87fbe9b212889d))
- *(lint)* Cleanup lint test structure ([#254](https://github.com/UnknownPlatypus/djangofmt/issues/254)) - ([5bf935e](https://github.com/UnknownPlatypus/djangofmt/commit/5bf935e549e2a21594e7af58658245a96d1b635d))

### ⚙️ Miscellaneous Tasks

- *(playground)* Upgrade all node dependencies to latest ([#252](https://github.com/UnknownPlatypus/djangofmt/issues/252)) - ([da82da2](https://github.com/UnknownPlatypus/djangofmt/commit/da82da25939d4903847c883b5159b6e77146956f))

### New Contributors ❤️

- @jonathan-s made their first contribution in [#200](https://github.com/UnknownPlatypus/djangofmt/pull/200)

## [0.2.6](https://github.com/UnknownPlatypus/djangofmt/compare/v0.2.5..v0.2.6) - 2026-02-10

[**Docs**](https://unknownplatypus.github.io/djangofmt/docs/) | [**Playground**](https://unknownplatypus.github.io/djangofmt/)

### ⛰️ Features

- *(config)* Implement loading options from a `pyproject.toml` file ([#186](https://github.com/UnknownPlatypus/djangofmt/issues/186)) - ([5df4a98](https://github.com/UnknownPlatypus/djangofmt/commit/5df4a989fe69114f6149a86a795de3ac684d2512))

### 🐛 Bug Fixes

- *(format)* Always swap wrapping quotes if attribute contains one ([#180](https://github.com/UnknownPlatypus/djangofmt/issues/180)) - ([0fd9ed3](https://github.com/UnknownPlatypus/djangofmt/commit/0fd9ed3e4a74939caba33eb9c7adac61c950b79e))
- *(format)* Fix formatting of `{% for %}`/`{% empty %}` blocks ([#178](https://github.com/UnknownPlatypus/djangofmt/issues/178)) - ([98f8ef7](https://github.com/UnknownPlatypus/djangofmt/commit/98f8ef7b0db7552b22bcfcc2eac6165ee5a4905b))

### 📚 Documentation

- *(format)* Documents workarounds for unsupported formatting ([#174](https://github.com/UnknownPlatypus/djangofmt/issues/174)) - ([15b1f32](https://github.com/UnknownPlatypus/djangofmt/commit/15b1f324df68e15b34e774ed3f023378f2de10f0))
- *(github)* Add Github issue template ([#173](https://github.com/UnknownPlatypus/djangofmt/issues/173)) - ([5821c07](https://github.com/UnknownPlatypus/djangofmt/commit/5821c0725d140382334ad2a48dfdbc439d7a1fbf))
- *(ignore)* Document ignore comment ([#172](https://github.com/UnknownPlatypus/djangofmt/issues/172)) - ([50a2fdd](https://github.com/UnknownPlatypus/djangofmt/commit/50a2fddb336570bf14173f90dd282fffd23adeef))

### 🧪 Testing

- *(ci)* Only build djangofmt in ci, not other workspace crates ([#170](https://github.com/UnknownPlatypus/djangofmt/issues/170)) - ([7bbbfc8](https://github.com/UnknownPlatypus/djangofmt/commit/7bbbfc8f66a3d5f018a55355366c6d72ea8db5b9))

## [0.2.5](https://github.com/UnknownPlatypus/djangofmt/compare/v0.2.3..v0.2.5) - 2026-01-12

[**Docs**](https://unknownplatypus.github.io/djangofmt/docs/) | [**Playground**](https://unknownplatypus.github.io/djangofmt/)

### ⛰️ Features

- *(debug)* Add debug logs on external formatter failures ([#143](https://github.com/UnknownPlatypus/djangofmt/issues/143)) - ([a5dda3b](https://github.com/UnknownPlatypus/djangofmt/commit/a5dda3bf4d11c94d0da4f667835d03074fb43cc2))
- *(format)* Don't format django multiline comment tags (`{% comment %}...{% endcomment %}`) ([#162](https://github.com/UnknownPlatypus/djangofmt/issues/162)) - ([6a6ca10](https://github.com/UnknownPlatypus/djangofmt/commit/6a6ca106c3009fa85d7555b7f019f6c6de500620))
- *(format)* Auto-sort css statements using `smacss` ordering and enforce `%` in keyframes ([#114](https://github.com/UnknownPlatypus/djangofmt/issues/114)) - ([1dd641f](https://github.com/UnknownPlatypus/djangofmt/commit/1dd641fedc4a07f826c95935d4ceb9d68e0a0070))
- *(format)* Auto-indent `<script>` tag content ([#115](https://github.com/UnknownPlatypus/djangofmt/issues/115)) - ([d959cf0](https://github.com/UnknownPlatypus/djangofmt/commit/d959cf060622a9e6b20f022e22b9c7ad538852f3))
- *(format)* Keep style attribute value on a single line ([#113](https://github.com/UnknownPlatypus/djangofmt/issues/113)) - ([7c2f33c](https://github.com/UnknownPlatypus/djangofmt/commit/7c2f33ca3987ed659dcfb91e4b13999bb4694dde))
- *(format)* Skip file parsing if there is a top-level `<!-- djangofmt:ignore -->` ([#112](https://github.com/UnknownPlatypus/djangofmt/issues/112)) - ([fcbab9b](https://github.com/UnknownPlatypus/djangofmt/commit/fcbab9b3cc2315b930314a78b4643dd23ce5ba10))
- *(format)* Improve formatting of `style` tags and attributes ([#111](https://github.com/UnknownPlatypus/djangofmt/issues/111)) - ([24920db](https://github.com/UnknownPlatypus/djangofmt/commit/24920db6e561ae3360d5b972dd66c7d8fe3b777b))
- *(lint)* Show source file name in lint diagnostics ([#159](https://github.com/UnknownPlatypus/djangofmt/issues/159)) - ([6ee8c96](https://github.com/UnknownPlatypus/djangofmt/commit/6ee8c963af1bdefad0c6509f4f672be371e397aa))
- *(lint)* Add building blocks for linting ([#144](https://github.com/UnknownPlatypus/djangofmt/issues/144)) - ([08a12a0](https://github.com/UnknownPlatypus/djangofmt/commit/08a12a05fb4037a8569b7ee10dfc49672058e682))
- *(playground)* Add a "Open issue on Github" button in the playground ([#163](https://github.com/UnknownPlatypus/djangofmt/issues/163)) - ([ae247b3](https://github.com/UnknownPlatypus/djangofmt/commit/ae247b36c92434aa643c48515b017262312378a8))
- *(playground)* Add a new collapsible panel to display linting errors ([#148](https://github.com/UnknownPlatypus/djangofmt/issues/148)) - ([067e4a0](https://github.com/UnknownPlatypus/djangofmt/commit/067e4a090d5a801cb31483461a90ed485797b4af))
- *(playground)* Add playground deploy to release workflow ([#124](https://github.com/UnknownPlatypus/djangofmt/issues/124)) - ([124b149](https://github.com/UnknownPlatypus/djangofmt/commit/124b14995a7a5611bc84f00d6493f1cddb1ca928))
- *(playground)* Add an online playground ([#118](https://github.com/UnknownPlatypus/djangofmt/issues/118)) - ([a655190](https://github.com/UnknownPlatypus/djangofmt/commit/a655190832fbd78e06c86f00cc5d7ecb3e7bb28b))
- *(playground)* Expose a wasm format command ([#122](https://github.com/UnknownPlatypus/djangofmt/issues/122)) - ([847d3d8](https://github.com/UnknownPlatypus/djangofmt/commit/847d3d830a66239ef06d07827a73ec7d4914aa1a))

### 🐛 Bug Fixes

- *(format)* Fixes handling of deprecated django `{% trans %}` tag ([#154](https://github.com/UnknownPlatypus/djangofmt/issues/154)) - ([8a7a64f](https://github.com/UnknownPlatypus/djangofmt/commit/8a7a64f041c7eecf4c04c84f3942a97e94ca578f))

### 🚜 Refactor

- *(cargo)* Switch to workspace setup ([#121](https://github.com/UnknownPlatypus/djangofmt/issues/121)) - ([ad75960](https://github.com/UnknownPlatypus/djangofmt/commit/ad75960ee2a7d972a92b7ec078837d9031ef451e))
- *(format)* Remove `line_col_to_offset` and use miette `SourceOffset::from_location` for error reporting ([#145](https://github.com/UnknownPlatypus/djangofmt/issues/145)) - ([f8acc95](https://github.com/UnknownPlatypus/djangofmt/commit/f8acc953b8d34dd382c7e86b54f013b9fb5130e8))

### ⚡ Performance

- *(allocator)* Use jemalloc on linux ([#128](https://github.com/UnknownPlatypus/djangofmt/issues/128)) - ([4e92927](https://github.com/UnknownPlatypus/djangofmt/commit/4e92927ac3f3de5b2629ddcda446bd3b7462357f))
- *(codspeed)* Setup Codspeed CI benchmarks ([#125](https://github.com/UnknownPlatypus/djangofmt/issues/125)) - ([0ea2b39](https://github.com/UnknownPlatypus/djangofmt/commit/0ea2b399dfd19c31b1d82c06c7cf10186f39456b))
- *(perf)* Update the `lto` and `codegen-units` benchmark script ([#126](https://github.com/UnknownPlatypus/djangofmt/issues/126)) - ([ccd7ede](https://github.com/UnknownPlatypus/djangofmt/commit/ccd7edea4eaa3e30b2a1017b833d498b34b95c34))

### 🧪 Testing

- *(binary-size)* Add cargo-bloat ([#136](https://github.com/UnknownPlatypus/djangofmt/issues/136)) - ([2855cb2](https://github.com/UnknownPlatypus/djangofmt/commit/2855cb24535d75daa40d224e307f14f7729f41d5))
- *(ci)* Skip codspeed in CI if no rust code changes ([#158](https://github.com/UnknownPlatypus/djangofmt/issues/158)) - ([8bf4432](https://github.com/UnknownPlatypus/djangofmt/commit/8bf4432cef316027a50324ce8dae9c54d480a459))
- *(ci)* Cancel outdated ci jobs - ([18595c3](https://github.com/UnknownPlatypus/djangofmt/commit/18595c32391b441601fdd13d668f8fb32325b68a))
- *(clippy)* Improve clippy configuration ([#156](https://github.com/UnknownPlatypus/djangofmt/issues/156)) - ([2de17b3](https://github.com/UnknownPlatypus/djangofmt/commit/2de17b3e70d3ec630e2f11cf28c3c5a86c949eaf))
- *(pre-commit)* Switch to a managed `dprint` pre-commit integration ([#138](https://github.com/UnknownPlatypus/djangofmt/issues/138)) - ([1ccfba0](https://github.com/UnknownPlatypus/djangofmt/commit/1ccfba0f23dd5069aeb9ca3eb2709a82edf2a966))
- *(pre-commit)* Simplify dprint discovery in pre-commit ([#137](https://github.com/UnknownPlatypus/djangofmt/issues/137)) - ([f4eb28c](https://github.com/UnknownPlatypus/djangofmt/commit/f4eb28c2ed38ac3a55f1aeb111749da32176b4ea))
- *(review)* Add coderrabit ([#141](https://github.com/UnknownPlatypus/djangofmt/issues/141)) - ([bac6128](https://github.com/UnknownPlatypus/djangofmt/commit/bac6128b6f38dc2ac77dee8e8b727ab5b30b5f84))

### ⚙️ Miscellaneous Tasks

- *(rust)* Bump rust version to 1.89 ([#110](https://github.com/UnknownPlatypus/djangofmt/issues/110)) - ([dcf68aa](https://github.com/UnknownPlatypus/djangofmt/commit/dcf68aa2b68ff01dc8e959d32b7b31fff6173cfe))
- Update release script - ([815fd8c](https://github.com/UnknownPlatypus/djangofmt/commit/815fd8c6470dfc19656b74412b72bf6f7a58e2bf))

### New Contributors ❤️

- @Mouarius made their first contribution in [#156](https://github.com/UnknownPlatypus/djangofmt/pull/156)

## [0.2.3](https://github.com/UnknownPlatypus/djangofmt/compare/v0.2.2..v0.2.3) - 2025-11-30

[**Docs**](https://unknownplatypus.github.io/djangofmt/docs/) | [**Playground**](https://unknownplatypus.github.io/djangofmt/)

### ⛰️ Features

- *(format)* Add pretty-printing of parse errors ([#104](https://github.com/UnknownPlatypus/djangofmt/issues/104)) - ([fee4ee3](https://github.com/UnknownPlatypus/djangofmt/commit/fee4ee3eacb748e77d6062813e044bace68adda1))
- *(format)* Add custom `<!-- djangofmt:ignore -->` directive ([#102](https://github.com/UnknownPlatypus/djangofmt/issues/102)) - ([c9b20bb](https://github.com/UnknownPlatypus/djangofmt/commit/c9b20bb6a8e22fb4c9bf9bde89a51154de04a1aa))
- *(format)* Improvements on inline node formatting ([#75](https://github.com/UnknownPlatypus/djangofmt/issues/75)) - ([58a4f8f](https://github.com/UnknownPlatypus/djangofmt/commit/58a4f8fa3479cbd33ecbe02b22c8792440b2560b))

### 🚜 Refactor

- *(error)* Remove `anyhow` and use custom error enum ([#100](https://github.com/UnknownPlatypus/djangofmt/issues/100)) - ([e39f71a](https://github.com/UnknownPlatypus/djangofmt/commit/e39f71a0c90086cc5f0080059b2dd201c37cf006))

### 🧪 Testing

- *(clippy)* Enable strict clippy rules ([#97](https://github.com/UnknownPlatypus/djangofmt/issues/97)) - ([cd7d6ea](https://github.com/UnknownPlatypus/djangofmt/commit/cd7d6eaaf0a42990fade3c813660b4f40fbf140e))
- *(ecosystem-check)* Fix ecosystem check comment urls and support codeberg.com ([#101](https://github.com/UnknownPlatypus/djangofmt/issues/101)) - ([337592c](https://github.com/UnknownPlatypus/djangofmt/commit/337592cb96d8d8e3a12f3992bddf87e77f9fe925))
- *(ecosystem-check)* Fix ecosystem-check and support other providers than github ([#99](https://github.com/UnknownPlatypus/djangofmt/issues/99)) - ([13e8bb2](https://github.com/UnknownPlatypus/djangofmt/commit/13e8bb28c4913d38fc2c1fe52b2ce8e1d9d30b94))

## [0.2.2](https://github.com/UnknownPlatypus/djangofmt/compare/v0.2.1..v0.2.2) - 2025-06-12

[**Docs**](https://unknownplatypus.github.io/djangofmt/docs/) | [**Playground**](https://unknownplatypus.github.io/djangofmt/)

### ⛰️ Features

- *(format)* Document svg files support ([#74](https://github.com/UnknownPlatypus/djangofmt/issues/74)) - ([60ba20f](https://github.com/UnknownPlatypus/djangofmt/commit/60ba20fc33a9ccbe6d908031e7513f974ee8c6d6))
- *(format)* Support unquoted attr value recovery for jinja tags & blocks ([#73](https://github.com/UnknownPlatypus/djangofmt/issues/73)) - ([ca7efc6](https://github.com/UnknownPlatypus/djangofmt/commit/ca7efc6c12c97cb8a9210c265569ad1a4b0f213d))

## [0.2.1](https://github.com/UnknownPlatypus/djangofmt/compare/v0.2.0..v0.2.1) - 2025-06-07

[**Docs**](https://unknownplatypus.github.io/djangofmt/docs/) | [**Playground**](https://unknownplatypus.github.io/djangofmt/)

### ⛰️ Features

- *(format)* Improve whitespace-sensitive node formatting ([#70](https://github.com/UnknownPlatypus/djangofmt/issues/70)) - ([cfaa21e](https://github.com/UnknownPlatypus/djangofmt/commit/cfaa21e2ca37134bb9e315b17a0bc6f82f6ac289))

## [0.2.0](https://github.com/UnknownPlatypus/djangofmt/compare/v0.1.0..v0.2.0) - 2025-05-23

[**Docs**](https://unknownplatypus.github.io/djangofmt/docs/) | [**Playground**](https://unknownplatypus.github.io/djangofmt/)

### ⛰️ Features

- *(cli)* Add `indent_width` cli parameter ([#48](https://github.com/UnknownPlatypus/djangofmt/issues/48)) - ([b0d0219](https://github.com/UnknownPlatypus/djangofmt/commit/b0d021948d564282f32b95688f0d31cbd6d8e633))
- *(fmt)* Never wrap opening tag with no attrs ([#20](https://github.com/UnknownPlatypus/djangofmt/issues/20)) - ([8fb993a](https://github.com/UnknownPlatypus/djangofmt/commit/8fb993a37f3ce1fbeb911eb91f8d92485a7db62c))
- *(format)* Converge in one pass formatting style attr ([#50](https://github.com/UnknownPlatypus/djangofmt/issues/50)) - ([fadce6b](https://github.com/UnknownPlatypus/djangofmt/commit/fadce6b31b8345543419eb0c6c5703e80810b2ec))

### 🐛 Bug Fixes

- *(cli)* Exit `1` on handled formatting failure - ([b7ebb78](https://github.com/UnknownPlatypus/djangofmt/commit/b7ebb789a865f62a58bb3b34cccaafea8f0e20e7))

### 🧪 Testing

- *(ecosystem-check)* Stability test + Integration test with djade ([#51](https://github.com/UnknownPlatypus/djangofmt/issues/51)) - ([449456e](https://github.com/UnknownPlatypus/djangofmt/commit/449456e3c2da1f643402772ee555dc34fa8af132))
- *(pre-commit.ci)* Enable `pre-commit.ci` ([#15](https://github.com/UnknownPlatypus/djangofmt/issues/15)) - ([5ce836f](https://github.com/UnknownPlatypus/djangofmt/commit/5ce836f701c8082bfe56ebdbba05a00cf8644e5b))

### ⚙️ Miscellaneous Tasks

- *(markup_fmt)* Bump `markup_fmt` to v0.20.0 ([#65](https://github.com/UnknownPlatypus/djangofmt/issues/65)) - ([070e2af](https://github.com/UnknownPlatypus/djangofmt/commit/070e2af30a1d888d66592324af5d77b10820b249))
- *(rust)* Rust 1.87 edition 2024 ([#55](https://github.com/UnknownPlatypus/djangofmt/issues/55)) - ([9aeb174](https://github.com/UnknownPlatypus/djangofmt/commit/9aeb174595bbb8d2da893dc41b3f4054368c71c9))

### New Contributors ❤️

- @renovate\[bot\] made their first contribution in [#64](https://github.com/UnknownPlatypus/djangofmt/pull/64)
- @pre-commit-ci\[bot\] made their first contribution in [#52](https://github.com/UnknownPlatypus/djangofmt/pull/52)

## [0.1.0](https://github.com/UnknownPlatypus/djangofmt/releases/tag/v0.1.0) - 2025-03-16

[**Docs**](https://unknownplatypus.github.io/djangofmt/docs/) | [**Playground**](https://unknownplatypus.github.io/djangofmt/)

### New Contributors ❤️

- @UnknownPlatypus made their first contribution
