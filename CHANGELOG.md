# Changelog

All notable changes to the zentract workspace are documented here. The format
follows [Keep a Changelog](https://keepachangelog.com/). All three crates
(`zentract-types`, `zentract-abi`, `zentract-api`) are pre-release at `0.1.0`,
unpublished, and version in lockstep.

## [Unreleased]

### Changed

- `tract-onnx` 0.22 → 0.22.3, `libloading` 0.9 → 0.9.0, `thiserror` 2 → 2.0.20
  (all previously truncated), and `zenutils-apidoc` 0.1.0 → 0.1.1 in the apidoc
  runner. Lockfile refreshed (56 packages); no zen-family crate moved.
- **`tract-onnx` deliberately held on the 0.22 line.** Every 0.23 release —
  0.23.0 through the current 0.23.5 — declares `rust-version = 1.91`, while this
  workspace declares `rust-version = "1.88"`. The `MSRV` CI job runs
  `cargo hack check --rust-version --workspace`, which is exactly the check that
  would fail. 0.22.3 (MSRV 1.85) is the newest release that fits, so the
  requirement moves within its line instead. Adopting 0.23 means raising the
  declared MSRV by three toolchain releases — an owner decision.

### zentract-api

#### Added
- Detached model handles — `ModelHandle::into_raw`, `InferenceEngine::infer_raw`,
  and `InferenceEngine::free_raw`: run inference on a raw `i64` handle that
  outlives the borrowing `ModelHandle`, freeing it explicitly with `free_raw`
  (55c4ee1).

### Workspace

#### Fixed
- **Pushes to `main` now cancel their superseded CI runs.** `ci.yml` keyed its
  concurrency group on `${{ github.head_ref || github.run_id }}`.
  `github.head_ref` is populated only for `pull_request` events, so on a push it
  was empty and the group fell through to `github.run_id` — unique per run, so no
  two pushes ever shared a group and `cancel-in-progress` could never fire. Every
  push started a full matrix that ran to completion even when several commits
  landed seconds apart. Now keyed on `${{ github.ref }}`, which is set for both
  event types (`refs/heads/main` on push, `refs/pull/N/merge` on a PR), so PR
  cancellation is unchanged and consecutive pushes supersede each other.

#### Added
- GitHub Actions CI: test matrix (Linux/macOS/Windows on x86-64, plus Linux and
  macOS on ARM64), clippy, rustfmt, and MSRV (1.88) checks (d11d230, 6de10f8).
- Versioned public-API surface snapshots regenerated on every test run, with a
  conservative ablation report (d4de5d2, 59ab16b).
- Per-package `exclude` lists and standard `.gitignore` entries so tooling noise
  stays out of published tarballs (4d2fc65, 3ebb5af).

#### Changed
- Public-API snapshot runner migrated to the CI-free zenutils-apidoc 0.1.0
  package (556d029).
- Dependencies refreshed (b1d6e11, fc1d8e0, efa45ab, 634b326).
- Dual licensing standardized as AGPL-3.0 OR Imazen Commercial (0f263f1).

#### Docs
- README overhaul: badge row, Quick start, detached-handle and threading notes,
  corrected dependency-tree claim, split crates.io README (`README.crates.md`),
  and the crosslink footer.
