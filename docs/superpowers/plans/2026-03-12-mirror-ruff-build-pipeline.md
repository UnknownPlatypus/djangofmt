# Mirror Ruff Build Pipeline Improvements — Implementation Plan

> **For agentic workers:** REQUIRED: Use superpowers:subagent-driven-development (if subagents available) or superpowers:executing-plans to implement this plan. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Adopt 6 ruff build pipeline improvements: explicit manylinux targets, `--compatibility pypi`, concurrency fix, Python bump, drop ppc64, add riscv64.

**Architecture:** All changes are in `.github/workflows/release-build-binaries.yml` (CI workflow) and `dist-workspace.toml` (cargo-dist config). No Rust code changes needed.

**Tech Stack:** GitHub Actions, maturin, cargo-dist

**Spec:** `docs/superpowers/specs/2026-03-12-mirror-ruff-build-pipeline-design.md`

---

## Task 1: Update env vars, concurrency, and Python version

**Files:**

- Modify: `.github/workflows/release-build-binaries.yml:29-39`

- [ ] **Step 1: Fix concurrency group**

Change lines 29-31 from:

```yaml
concurrency:
  group: ${{ github.workflow }}-${{ github.ref }}
  cancel-in-progress: true
```

To:

```yaml
concurrency:
  group: build-binaries-${{ github.ref }}
  cancel-in-progress: ${{ github.event_name == 'pull_request' }}
```

- [ ] **Step 2: Bump Python version**

Change line 36 from:

```yaml
PYTHON_VERSION: "3.11"
```

To:

```yaml
PYTHON_VERSION: "3.13"
```

- [ ] **Step 3: Commit**

```bash
git add .github/workflows/release-build-binaries.yml
git commit -m "ci: fix concurrency group and bump Python to 3.13

Hard-code workflow name to prevent cross-workflow cancellation.
Only cancel in-progress for PRs, not release runs.
Bump build Python from 3.11 to 3.13 to match ruff."
```

---

## Task 2: Explicit manylinux targets for `linux` job

**Files:**

- Modify: `.github/workflows/release-build-binaries.yml:83-88`

- [ ] **Step 1: Pin manylinux version**

Change line 87 in the `linux` job's "Build wheels" step from:

```yaml
manylinux: auto
```

To:

```yaml
manylinux: "2_17"
```

- [ ] **Step 2: Add `--compatibility pypi` to linux build args**

Change line 88 from:

```yaml
args: --release --locked --out dist
```

To:

```yaml
args: --release --locked --out dist --compatibility pypi
```

- [ ] **Step 3: Commit**

```bash
git add .github/workflows/release-build-binaries.yml
git commit -m "ci: pin linux manylinux to 2_17 and add --compatibility pypi"
```

---

## Task 3: Update `linux-cross` matrix — explicit manylinux, drop ppc64, add riscv64

**Files:**

- Modify: `.github/workflows/release-build-binaries.yml:120-196`

- [ ] **Step 1: Add `manylinux` field to each matrix entry and drop ppc64**

Replace the entire `linux-cross` matrix (lines 123-143) with:

```yaml
platform:
  - target: aarch64-unknown-linux-gnu
    arch: aarch64
    manylinux: "2_17"
    # see https://github.com/astral-sh/ruff/issues/3791
    # and https://github.com/gnzlbg/jemallocator/issues/170#issuecomment-1503228963
    maturin_docker_options: -e JEMALLOC_SYS_WITH_LG_PAGE=16
  - target: armv7-unknown-linux-gnueabihf
    arch: armv7
    manylinux: "2_17"
  - target: s390x-unknown-linux-gnu
    arch: s390x
    manylinux: "2_17"
  - target: powerpc64le-unknown-linux-gnu
    arch: ppc64le
    manylinux: "2_17"
    # see https://github.com/astral-sh/ruff/issues/10073
    maturin_docker_options: -e JEMALLOC_SYS_WITH_LG_PAGE=16
  - target: riscv64gc-unknown-linux-gnu
    arch: riscv64
    manylinux: "2_31"
    maturin_docker_options: -e JEMALLOC_SYS_WITH_LG_PAGE=16
  - target: arm-unknown-linux-musleabihf
    arch: arm
    manylinux: auto
```

- [ ] **Step 2: Update maturin-action to use per-entry manylinux**

Change the maturin-action step's `manylinux` parameter (line 156) from:

```yaml
manylinux: auto
```

To:

```yaml
manylinux: ${{ matrix.platform.manylinux }}
```

- [ ] **Step 3: Add `--compatibility pypi` to linux-cross build args**

Change the maturin-action `args` (line 158) from:

```yaml
args: --release --locked --out dist
```

To:

```yaml
args: --release --locked --out dist --compatibility pypi
```

- [ ] **Step 4: Add `before-script-linux` to maturin-action for riscv64 libatomic1**

Add the `before-script-linux` parameter to the maturin-action step (after the `args` line):

```yaml
before-script-linux: |
  if command -v apt-get &> /dev/null; then
    apt-get update && apt-get install -y libatomic1
  fi
```

- [ ] **Step 5: Update run-on-arch-action skip condition**

Change the `if` condition on the test step (line 160) from:

```yaml
if: ${{ matrix.platform.arch != 'ppc64' && matrix.platform.arch != 'ppc64le' }}
```

To:

```yaml
if: ${{ matrix.platform.arch != 'ppc64le' && matrix.platform.arch != 'riscv64' }}
```

- [ ] **Step 6: Commit**

```bash
git add .github/workflows/release-build-binaries.yml
git commit -m "ci: explicit manylinux in linux-cross, drop ppc64, add riscv64

- Add per-entry manylinux field to linux-cross matrix
- Drop powerpc64-unknown-linux-gnu (big-endian, dead platform)
- Add riscv64gc-unknown-linux-gnu with manylinux 2_31
- Install libatomic1 for riscv64 builds
- Add --compatibility pypi pre-upload validation"
```

---

## Task 4: Add `--compatibility pypi` to remaining jobs

**Files:**

- Modify: `.github/workflows/release-build-binaries.yml` — musllinux, musllinux-cross, macos-x86_64, macos-aarch64, windows jobs

- [ ] **Step 1: musllinux job**

Change the maturin-action `args` (line 218) from:

```yaml
args: --release --locked --out dist
```

To:

```yaml
args: --release --locked --out dist --compatibility pypi
```

- [ ] **Step 2: musllinux-cross job**

Change the maturin-action `args` (line 277) from:

```yaml
args: --release --locked --out dist
```

To:

```yaml
args: --release --locked --out dist --compatibility pypi
```

- [ ] **Step 3: macos-x86_64 job**

Change the maturin-action `args` (line 331) from:

```yaml
args: --release --locked --out dist
```

To:

```yaml
args: --release --locked --out dist --compatibility pypi
```

- [ ] **Step 4: macos-aarch64 job**

Change the maturin-action `args` (line 370) from:

```yaml
args: --release --locked --out dist
```

To:

```yaml
args: --release --locked --out dist --compatibility pypi
```

- [ ] **Step 5: windows job**

Change the maturin-action `args` (line 422) from:

```yaml
args: --release --locked --out dist
```

To:

```yaml
args: --release --locked --out dist --compatibility pypi
```

- [ ] **Step 6: Commit**

```bash
git add .github/workflows/release-build-binaries.yml
git commit -m "ci: add --compatibility pypi to all remaining wheel build jobs"
```

---

## Task 5: Update `dist-workspace.toml` targets

**Files:**

- Modify: `dist-workspace.toml:25-43`

- [ ] **Step 1: Drop ppc64 and add riscv64**

In the `targets` list, remove this line:

```toml
"powerpc64-unknown-linux-gnu",
```

And add this line (after `powerpc64le-unknown-linux-gnu`):

```toml
"riscv64gc-unknown-linux-gnu",
```

- [ ] **Step 2: Commit**

```bash
git add dist-workspace.toml
git commit -m "ci: update dist targets — drop ppc64, add riscv64"
```

---

## Task 6: Verify

- [ ] **Step 1: Review the full diff**

```bash
git log --oneline main..HEAD
git diff main..HEAD
```

Verify:

- 5 commits total
- `release-build-binaries.yml`: concurrency fix, Python 3.13, explicit manylinux everywhere, `--compatibility pypi` on all 7 build jobs, ppc64 removed, riscv64 added with libatomic1 and skip condition
- `dist-workspace.toml`: ppc64 removed, riscv64 added

- [ ] **Step 2: Validate YAML syntax**

```bash
python -c "import yaml; yaml.safe_load(open('.github/workflows/release-build-binaries.yml'))"
```

- [ ] **Step 3: Trigger a dry-run release to test**

```bash
gh workflow run release.yml --field tag=dry-run
```
