# Mirror Ruff's Build Pipeline Improvements

**Date:** 2026-03-12
**Delivery:** Single PR with all 6 changes

## Context

Ruff (astral-sh/ruff) made several build pipeline improvements in January-March 2026. This spec adapts all of them to djangofmt's `release-build-binaries.yml` and `dist-workspace.toml`.

**References:**

- [ty PR #2393](https://github.com/astral-sh/ty/pull/2393) — Explicit manylinux + `--compatibility pypi`
- [ruff PR #22477](https://github.com/astral-sh/ruff/pull/22477) — Same for ruff
- [ruff commit 9aaa82d](https://github.com/astral-sh/ruff/commit/9aaa82d) — RISC-V support
- [ruff PR #22766](https://github.com/astral-sh/ruff/pull/22766) — Drop ppc64 big-endian
- [ruff PR #23431](https://github.com/astral-sh/ruff/pull/23431) — Concurrency fix

## Files Modified

| File                                           | Changes                        |
| ---------------------------------------------- | ------------------------------ |
| `.github/workflows/release-build-binaries.yml` | All 6 changes                  |
| `dist-workspace.toml`                          | Drop ppc64, add riscv64 target |

No Rust code changes — `Cargo.toml` and `main.rs` already include `riscv64` in the jemalloc target arch list.

## Change 1: Explicit manylinux/musllinux Targets

**Rationale:** `manylinux: auto` lets maturin guess the minimum glibc version. If maturin's heuristics change, wheels could silently shift compatibility. Pinning makes changes intentional.

### `linux` job

`manylinux: auto` → `manylinux: 2_17` (applies to both x86_64 and i686 targets).

### `linux-cross` job

Add a `manylinux` field to each matrix entry and reference it as `${{ matrix.platform.manylinux }}` in the maturin-action step:

| Target                              | manylinux                                                                   |
| ----------------------------------- | --------------------------------------------------------------------------- |
| `aarch64-unknown-linux-gnu`         | `2_17`                                                                      |
| `armv7-unknown-linux-gnueabihf`     | `2_17`                                                                      |
| `s390x-unknown-linux-gnu`           | `2_17`                                                                      |
| `powerpc64le-unknown-linux-gnu`     | `2_17`                                                                      |
| `arm-unknown-linux-musleabihf`      | `auto` (new per-entry field — musl cross target, container handles tagging) |
| `riscv64gc-unknown-linux-gnu` (new) | `2_31` (minimum available for riscv64)                                      |

### `musllinux` and `musllinux-cross` jobs

Already use `musllinux_1_2` — no change needed.

## Change 2: `--compatibility pypi` Pre-upload Validation

**Rationale:** Maturin 1.11.5+ can validate wheels meet PyPI compatibility requirements before upload. Catches bad wheel tags early instead of failing at publish time.

Add `--compatibility pypi` to the `args` of every `maturin build` step in these 7 jobs (all except `sdist` which uses `maturin sdist`):

1. `linux` (matrix: x86_64, i686)
2. `linux-cross` (matrix: 6 targets)
3. `musllinux` (matrix: x86_64, i686)
4. `musllinux-cross` (matrix: aarch64, armv7)
5. `macos-x86_64`
6. `macos-aarch64`
7. `windows` (matrix: x86_64, i686, aarch64)

Before: `args: --release --locked --out dist`
After: `args: --release --locked --out dist --compatibility pypi`

## Change 3: Fix Concurrency Groups

**Rationale:** `${{ github.workflow }}` can cause unrelated workflows to cancel each other when they share the same computed name. Release runs should never be cancelled mid-flight.

Before:

```yaml
concurrency:
  group: ${{ github.workflow }}-${{ github.ref }}
  cancel-in-progress: true
```

After:

```yaml
concurrency:
  group: build-binaries-${{ github.ref }}
  cancel-in-progress: ${{ github.event_name == 'pull_request' }}
```

## Change 4: Python 3.11 → 3.13

**Rationale:** Keep the build environment current. Matches ruff.

`PYTHON_VERSION: "3.11"` → `PYTHON_VERSION: "3.13"`

## Change 5: Drop `powerpc64-unknown-linux-gnu` (Big-Endian)

**Rationale:** Platform is dead, only supported on one exact manylinux version ([pypa/auditwheel#669](https://github.com/pypa/auditwheel/issues/669)), no known users. `powerpc64le` (little-endian) is kept. Easy to re-add if needed.

- Remove `powerpc64-unknown-linux-gnu` entry from `linux-cross` matrix in `release-build-binaries.yml`
- Remove `"powerpc64-unknown-linux-gnu"` from `dist-workspace.toml` targets list

## Change 6: Add `riscv64gc-unknown-linux-gnu`

### `release-build-binaries.yml`

Add to `linux-cross` matrix:

```yaml
- target: riscv64gc-unknown-linux-gnu
  arch: riscv64
  manylinux: "2_31"
  maturin_docker_options: -e JEMALLOC_SYS_WITH_LG_PAGE=16
```

The riscv64 build needs `libatomic1` installed in the Docker container (required for atomic operations on riscv64). Add `before-script-linux` to the maturin-action step:

```yaml
before-script-linux: |
  if command -v apt-get &> /dev/null; then
    apt-get update && apt-get install -y libatomic1
  fi
```

Update the `run-on-arch-action` skip condition. Currently it skips `ppc64` and `ppc64le`:

```yaml
# Before (ppc64 entry is being removed from matrix, so clean up its check too)
if: ${{ matrix.platform.arch != 'ppc64' && matrix.platform.arch != 'ppc64le' }}

# After
if: ${{ matrix.platform.arch != 'ppc64le' && matrix.platform.arch != 'riscv64' }}
```

### `dist-workspace.toml`

Add `"riscv64gc-unknown-linux-gnu"` to the targets list.

## Summary

| Aspect                | Before                       | After                                |
| --------------------- | ---------------------------- | ------------------------------------ |
| manylinux pinning     | `auto`                       | Explicit per-arch (`2_17`, `2_31`)   |
| Pre-upload validation | None                         | `--compatibility pypi` on all wheels |
| Concurrency           | Dynamic name, always cancels | Hard-coded name, cancel only on PR   |
| Python                | 3.11                         | 3.13                                 |
| ppc64 (big-endian)    | Built                        | Dropped                              |
| riscv64               | Not built                    | Built (manylinux 2_31)               |
| Target count          | 17                           | 17 (-1 +1)                           |

## Verification

Test with a dry-run release dispatch (`tag: dry-run`) to confirm all matrix builds succeed before merging.
