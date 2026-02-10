# Changelog

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
- *(perf)* Update the `lto` and `codegen-units` benchmark script ([#126](https://github.com/UnknownPlatypus/djangofmt/issues/126)) - ([ccd7edge](https://github.com/UnknownPlatypus/djangofmt/commit/ccd7edea4eaa3e30b2a1017b833d498b34b95c34))

### 🧪 Testing

- *(binary-size)* Add cargo-bloat ([#136](https://github.com/UnknownPlatypus/djangofmt/issues/136)) - ([2855cb2](https://github.com/UnknownPlatypus/djangofmt/commit/2855cb24535d75daa40d224e307f14f7729f41d5))
- *(ci)* Skip codspeed in CI if no rust code changes ([#158](https://github.com/UnknownPlatypus/djangofmt/issues/158)) - ([8bf4432](https://github.com/UnknownPlatypus/djangofmt/commit/8bf4432cef316027a50324ce8dae9c54d480a459))
- *(ci)* Cancel outdated ci jobs - ([18595c3](https://github.com/UnknownPlatypus/djangofmt/commit/18595c32391b441601fdd13d668f8fb32325b68a))
- *(clippy)* Improve clippy configuration ([#156](https://github.com/UnknownPlatypus/djangofmt/issues/156)) - ([2de17b3](https://github.com/UnknownPlatypus/djangofmt/commit/2de17b3e70d3ec630e2f11cf28c3c5a86c949eaf))
- *(pre-commit)* Switch to a managed `dprint` pre-commit integration ([#138](https://github.com/UnknownPlatypus/djangofmt/issues/138)) - ([1ccfba0](https://github.com/UnknownPlatypus/djangofmt/commit/1ccfba0f23dd5069aeb9ca3eb2709a82edf2a966))
- *(pre-commit)* Simplify dprint discovery in pre-commit ([#137](https://github.com/UnknownPlatypus/djangofmt/issues/137)) - ([f4eb28c](https://github.com/UnknownPlatypus/djangofmt/commit/f4eb28c2ed38ac3a55f1aeb111749da32176b4ea))
- *(review)* Add coderrabit ([#141](https://github.com/UnknownPlatypus/djangofmt/issues/141)) - ([bac6128](https://github.com/UnknownPlatypus/djangofmt/commit/bac6128b6f38dc2ac77dee8e8b727ab5b30b5f84))

## New Contributors ❤️

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

## New Contributors ❤️

- @renovate[bot] made their first contribution in [#64](https://github.com/UnknownPlatypus/djangofmt/pull/64)
- @pre-commit-ci[bot] made their first contribution in [#52](https://github.com/UnknownPlatypus/djangofmt/pull/52)

## [0.1.0] - 2025-03-16

## New Contributors ❤️

- @UnknownPlatypus made their first contribution
