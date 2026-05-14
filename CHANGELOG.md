# Changelog

## Unreleased

### ⛰️ Features

- *(lint)* Add `select` / `ignore` rule-selection configuration with preview/stability gating

## [0.2.10](https://github.com/UnknownPlatypus/djangofmt/compare/v0.2.9..v0.2.10) - 2026-06-03

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

## [0.2.5](https://github.com/UnknownPlatypus/djangofmt/compare/v0.2.4..v0.2.5) - 2026-01-12

### ⛰️ Features

- *(debug)* Add debug logs on external formatter failures ([#143](https://github.com/UnknownPlatypus/djangofmt/issues/143)) - ([a5dda3b](https://github.com/UnknownPlatypus/djangofmt/commit/a5dda3bf4d11c94d0da4f667835d03074fb43cc2))
- *(format)* Don't format django multiline comment tags (`{% comment %}...{% endcomment %}`) ([#162](https://github.com/UnknownPlatypus/djangofmt/issues/162)) - ([6a6ca10](https://github.com/UnknownPlatypus/djangofmt/commit/6a6ca106c3009fa85d7555b7f019f6c6de500620))
- *(lint)* Show source file name in lint diagnostics ([#159](https://github.com/UnknownPlatypus/djangofmt/issues/159)) - ([6ee8c96](https://github.com/UnknownPlatypus/djangofmt/commit/6ee8c963af1bdefad0c6509f4f672be371e397aa))
- *(lint)* Add building blocks for linting ([#144](https://github.com/UnknownPlatypus/djangofmt/issues/144)) - ([08a12a0](https://github.com/UnknownPlatypus/djangofmt/commit/08a12a05fb4037a8569b7ee10dfc49672058e682))
- *(playground)* Add a "Open issue on Github" button in the playground ([#163](https://github.com/UnknownPlatypus/djangofmt/issues/163)) - ([ae247b3](https://github.com/UnknownPlatypus/djangofmt/commit/ae247b36c92434aa643c48515b017262312378a8))
- *(playground)* Add a new collapsible panel to display linting errors ([#148](https://github.com/UnknownPlatypus/djangofmt/issues/148)) - ([067e4a0](https://github.com/UnknownPlatypus/djangofmt/commit/067e4a090d5a801cb31483461a90ed485797b4af))

### 🐛 Bug Fixes

- *(format)* Fixes handling of deprecated django `{% trans %}` tag ([#154](https://github.com/UnknownPlatypus/djangofmt/issues/154)) - ([8a7a64f](https://github.com/UnknownPlatypus/djangofmt/commit/8a7a64f041c7eecf4c04c84f3942a97e94ca578f))

### 🚜 Refactor

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

### New Contributors ❤️

- @Mouarius made their first contribution in [#156](https://github.com/UnknownPlatypus/djangofmt/pull/156)

## [0.2.4](https://github.com/UnknownPlatypus/djangofmt/compare/v0.2.3..v0.2.4) - 2025-12-15

### ⛰️ Features

- *(format)* Auto-sort css statements using `smacss` ordering and enforce `%` in keyframes ([#114](https://github.com/UnknownPlatypus/djangofmt/issues/114)) - ([1dd641f](https://github.com/UnknownPlatypus/djangofmt/commit/1dd641fedc4a07f826c95935d4ceb9d68e0a0070))
- *(format)* Auto-indent `<script>` tag content ([#115](https://github.com/UnknownPlatypus/djangofmt/issues/115)) - ([d959cf0](https://github.com/UnknownPlatypus/djangofmt/commit/d959cf060622a9e6b20f022e22b9c7ad538852f3))
- *(format)* Keep style attribute value on a single line ([#113](https://github.com/UnknownPlatypus/djangofmt/issues/113)) - ([7c2f33c](https://github.com/UnknownPlatypus/djangofmt/commit/7c2f33ca3987ed659dcfb91e4b13999bb4694dde))
- *(format)* Skip file parsing if there is a top-level `<!-- djangofmt:ignore -->` ([#112](https://github.com/UnknownPlatypus/djangofmt/issues/112)) - ([fcbab9b](https://github.com/UnknownPlatypus/djangofmt/commit/fcbab9b3cc2315b930314a78b4643dd23ce5ba10))
- *(format)* Improve formatting of `style` tags and attributes ([#111](https://github.com/UnknownPlatypus/djangofmt/issues/111)) - ([24920db](https://github.com/UnknownPlatypus/djangofmt/commit/24920db6e561ae3360d5b972dd66c7d8fe3b777b))
- *(playground)* Add playground deploy to release workflow ([#124](https://github.com/UnknownPlatypus/djangofmt/issues/124)) - ([124b149](https://github.com/UnknownPlatypus/djangofmt/commit/124b14995a7a5611bc84f00d6493f1cddb1ca928))
- *(playground)* Add an online playground ([#118](https://github.com/UnknownPlatypus/djangofmt/issues/118)) - ([a655190](https://github.com/UnknownPlatypus/djangofmt/commit/a655190832fbd78e06c86f00cc5d7ecb3e7bb28b))
- *(playground)* Expose a wasm format command ([#122](https://github.com/UnknownPlatypus/djangofmt/issues/122)) - ([847d3d8](https://github.com/UnknownPlatypus/djangofmt/commit/847d3d830a66239ef06d07827a73ec7d4914aa1a))

### 🚜 Refactor

- *(cargo)* Switch to workspace setup ([#121](https://github.com/UnknownPlatypus/djangofmt/issues/121)) - ([ad75960](https://github.com/UnknownPlatypus/djangofmt/commit/ad75960ee2a7d972a92b7ec078837d9031ef451e))

### ⚙️ Miscellaneous Tasks

- *(rust)* Bump rust version to 1.89 ([#110](https://github.com/UnknownPlatypus/djangofmt/issues/110)) - ([dcf68aa](https://github.com/UnknownPlatypus/djangofmt/commit/dcf68aa2b68ff01dc8e959d32b7b31fff6173cfe))
- Update release script - ([815fd8c](https://github.com/UnknownPlatypus/djangofmt/commit/815fd8c6470dfc19656b74412b72bf6f7a58e2bf))

## [0.2.3](https://github.com/UnknownPlatypus/djangofmt/compare/v0.2.2..v0.2.3) - 2025-11-30

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

### ⛰️ Features

- *(format)* Document svg files support ([#74](https://github.com/UnknownPlatypus/djangofmt/issues/74)) - ([60ba20f](https://github.com/UnknownPlatypus/djangofmt/commit/60ba20fc33a9ccbe6d908031e7513f974ee8c6d6))
- *(format)* Support unquoted attr value recovery for jinja tags & blocks ([#73](https://github.com/UnknownPlatypus/djangofmt/issues/73)) - ([ca7efc6](https://github.com/UnknownPlatypus/djangofmt/commit/ca7efc6c12c97cb8a9210c265569ad1a4b0f213d))

## [0.2.1](https://github.com/UnknownPlatypus/djangofmt/compare/v0.2.0..v0.2.1) - 2025-06-07

### ⛰️ Features

- *(format)* Improve whitespace-sensitive node formatting ([#70](https://github.com/UnknownPlatypus/djangofmt/issues/70)) - ([cfaa21e](https://github.com/UnknownPlatypus/djangofmt/commit/cfaa21e2ca37134bb9e315b17a0bc6f82f6ac289))

## [0.2.0](https://github.com/UnknownPlatypus/djangofmt/compare/v0.1.0..v0.2.0) - 2025-05-23

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

### New Contributors ❤️

- @UnknownPlatypus made their first contribution
