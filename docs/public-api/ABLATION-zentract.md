# zentract public-API ablation report

**Date:** 2026-06-11
**Snapshot commit:** d4de5d2
**Crates analyzed:** `zentract-types` (66 items) + `zentract-abi` (6 items) + `zentract-api` (65 items)
**Grep template:** `ugrep -r --include="*.rs" --include="*.toml" "<symbol>" /home/lilith/work/ --exclude-dir=target --exclude-dir=.jj`

## Consumer context

- `zentract-api`: consumed by `zensally/crates/zensally-zentract` — uses `InferenceEngine`, `free_raw`, `infer_raw`, `TensorMeta`.
- `zentract-types`: consumed transitively via zentract-api by zensally-zentract.
- `zentract-abi`: the C ABI cdylib layer — all 6 items are `#[no_mangle]` C functions; the entire surface is the dlopen plugin contract.

## Summary

**0 items flagged for action.**

### zentract-abi (6 items)

All 6 items are `#[no_mangle] pub extern "C"` functions forming the dlopen plugin contract: `zentract_abi_version`, `zentract_free`, `zentract_infer`, `zentract_load`, `zentract_output_count`. Per the mission brief: abi = KEEP wholesale. These are the ABI boundary that `zentract-api` loads at runtime via `libloading`. Stable by design.

### zentract-types (66 items)

Three types + two constants:
- `DType` (`#[repr(C)]` enum, 4 variants) — required by ABI
- `ErrorCode` (`#[repr(i32)]` enum, 6 variants) — required by ABI
- `TensorMeta` (`#[repr(C)]` struct with pub fields: `dtype`, `ndim`, `shape`) — required by ABI; pub fields needed because this is a C-compatible struct passed through the ABI boundary
- `ABI_VERSION: u32` — version constant
- `MAX_NDIM: usize` — shape array bound constant

All items are required by the ABI contract. No issues.

### zentract-api (65 items)

- `InferenceEngine` — `load()`, `load_onnx()`, `infer_raw()`, `free_raw()`. The `infer_raw` / `free_raw` escape hatches are confirmed consumed by `zensally-zentract/microsalnet.rs` and `ultraface.rs`, which manage raw handles directly for performance. Intentional.
- `ModelHandle<'e>` — `infer()`, `into_raw()`, `output_count()`. `into_raw()` exposes the raw handle i64 — escape hatch for callers that need manual handle management (same as `free_raw`).
- `InferOutput { pub data: Vec<f32>, pub meta: TensorMeta }` — struct with pub fields. Consumers read `.data` and `.meta` directly. No constructor needed; fields are the output. Intentional pub struct.
- `Error` enum with `LoadLibrary(libloading::error::Error)` — leaks `libloading::error::Error` as a public variant field. This is standard practice for error enums wrapping library errors. Not a concern.

## Flagged items

| # | Item | Category | Proposal | Confidence |
|---|------|----------|----------|------------|
| — | (none) | — | — | — |

**0 flagged. 0 % of surface.**

## Digest

All three zentract crates have minimal, intentional surfaces. zentract-abi is KEEP wholesale (ABI contract). zentract-types contains only C-ABI-compatible shared types. zentract-api exposes exactly what zensally-zentract consumes: `InferenceEngine` with both ergonomic (`ModelHandle`) and raw (`infer_raw`/`free_raw`) access paths. The `InferOutput` pub fields are needed by consumers. No leaks, no accidental exposures.
