# Changelog

All notable changes to the zentract workspace are documented here. The format
follows [Keep a Changelog](https://keepachangelog.com/). All three crates
(`zentract-types`, `zentract-abi`, `zentract-api`) are pre-release at `0.1.0`,
unpublished, and version in lockstep.

## [Unreleased]

### zentract-api

#### Added
- Detached model handles — `ModelHandle::into_raw`, `InferenceEngine::infer_raw`,
  and `InferenceEngine::free_raw`: run inference on a raw `i64` handle that
  outlives the borrowing `ModelHandle`, freeing it explicitly with `free_raw`
  (55c4ee1).

### Workspace

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
