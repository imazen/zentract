# zentract [![CI](https://img.shields.io/github/actions/workflow/status/imazen/zentract/ci.yml?style=flat-square&label=CI)](https://github.com/imazen/zentract/actions/workflows/ci.yml) [![crates.io](https://img.shields.io/crates/v/zentract-api?style=flat-square)](https://crates.io/crates/zentract-api) [![lib.rs](https://img.shields.io/crates/v/zentract-api?style=flat-square&label=lib.rs&color=blue)](https://lib.rs/crates/zentract-api) [![docs.rs](https://img.shields.io/docsrs/zentract-api?style=flat-square)](https://docs.rs/zentract-api) [![MSRV](https://img.shields.io/badge/MSRV-1.88-blue?style=flat-square)](https://doc.rust-lang.org/cargo/reference/manifest.html#the-rust-version-field) [![license](https://img.shields.io/badge/license-AGPL--3.0%20%2F%20Commercial-blue?style=flat-square)](#license)

zentract puts the [tract](https://github.com/sonos/tract) ONNX inference engine behind a C ABI in a `cdylib`, so your application loads it at runtime with `dlopen` instead of statically linking it. Your binary depends only on the small `zentract-api` host loader (`libloading` + `thiserror`); tract and its large transitive tree compile once, into the plugin, and stay out of your build.

## Quick start

```toml
[dependencies]
zentract-api = "0.1.0"   # host loader only — no tract dependency
```

Build the plugin once (`cargo build --release -p zentract-abi` produces `libzentract_abi.so` / `.dll` / `.dylib`), ship it next to your binary, then load and run a model:

```rust
use zentract_api::{InferenceEngine, TensorMeta};

// Load the plugin once, at startup.
let engine = InferenceEngine::load("libzentract_abi.so")?;

// Load an ONNX model with a fixed input shape.
let onnx = std::fs::read("model.onnx")?;
let input_shape = TensorMeta::f32_shape(&[1, 3, 320, 320]);
let model = engine.load_onnx(&onnx, input_shape)?;

// f32 tensor in, f32 tensor out.
let input: Vec<f32> = vec![0.0; 3 * 320 * 320]; // N*C*H*W elements
let output = model.infer(&input, 0)?;           // output_index = 0
let scores: &[f32] = &output.data;              // output.meta holds the shape
```

`ModelHandle` frees its model when dropped; the `InferenceEngine` stays loaded as long as you hold it.

## Why

tract-onnx is a well-maintained, pure-Rust ONNX runtime — but it's a large dependency. The workspace lockfile resolves to ~140 crates (the bulk of them tract's seven sub-crates and their dependencies), and that tree dominates compile time for any project that links it. If all you need is "f32 tensor in, f32 tensor out," there's no reason to rebuild the engine into every consumer.

zentract draws the line at a C ABI: the engine lives in a `cdylib`, your app links only the loader. You trade a `dlopen` call and a copied-out output buffer for a small, fast-compiling host crate and a plugin you can rebuild and distribute independently.

## Crates

| Crate | Type | Depends on | Role |
|-------|------|-----------|------|
| `zentract-types` | lib · `no_std` · `forbid(unsafe_code)` | — | Shared `#[repr(C)]` FFI types: `TensorMeta`, `DType`, `ErrorCode`, `ABI_VERSION` |
| `zentract-abi` | `cdylib` | tract-onnx 0.22 | The plugin: links tract, exports the `extern "C"` entry points |
| `zentract-api` | lib · `deny(unsafe_code)` | `libloading`, `thiserror` | Host-side loader; no tract dependency |

## Detached handles

By default a `ModelHandle` borrows its `InferenceEngine` and frees the model on drop — the safe, common case. When you need a model to outlive that borrow (store it in a struct, hand it across an API boundary, or manage its lifetime yourself), detach it to a raw `i64` handle:

```rust
let model = engine.load_onnx(&onnx, input_shape)?;
let raw: i64 = model.into_raw();          // model is NOT freed on drop

// ...later, run inference directly on the raw handle...
let output = engine.infer_raw(raw, &input, 0)?;

// You now own the lifetime — free it explicitly.
engine.free_raw(raw);
```

`into_raw` suppresses the `Drop`, so a matching `free_raw` is mandatory or the model leaks inside the plugin.

## Errors

`zentract-api` returns a single `Error` enum (via `thiserror`):

- `LoadLibrary` — the `cdylib` failed to open or is missing an export
- `AbiMismatch { expected, actual }` — plugin built against a different `ABI_VERSION`
- `ModelLoad(code)` — ONNX parse/optimize failed
- `Inference(code)` — a run failed (e.g. shape mismatch)
- `InvalidHandle` — the handle doesn't refer to a loaded model

`ModelHandle::output_count()` reports how many outputs a model exposes, so you can validate an `output_index` before calling `infer`.

## Threading

The plugin keeps loaded models in thread-local storage, so a model handle is only valid on the thread that loaded it — load and run each model on the same thread. `InferenceEngine` itself is `Send + Sync` and may be shared across threads; it's the per-thread handles that you must not move between threads.

## ABI contract

The plugin exports five `extern "C"` functions (defined in [`zentract-abi/src/lib.rs`](https://github.com/imazen/zentract/blob/main/zentract-abi/src/lib.rs)):

```c
uint32_t zentract_abi_version(void);
int64_t  zentract_load(const uint8_t *onnx, size_t len, const TensorMeta *input); // handle >= 0, else negative ErrorCode
int32_t  zentract_infer(int64_t handle, const float *input, size_t len, uint32_t output_index,
                        const float **out_data, size_t *out_len, TensorMeta *out_meta); // 0 = Ok
int32_t  zentract_output_count(int64_t handle);
void     zentract_free(int64_t handle);
```

`out_data` points into memory owned by the plugin and is valid only until the next `zentract_infer` or `zentract_free` on the same handle. `zentract-api` copies it into an owned `Vec<f32>` before returning, so host code never holds a dangling pointer. `ABI_VERSION` (currently `1`) is checked at load time and lives in `zentract-types`; bump it on any breaking change to these signatures.

## Building

Both plugin and host live in one workspace:

```sh
cargo build --release            # builds all three crates
# target/release/libzentract_abi.so   <- the plugin; ship this alongside your binary
```

Input tensors must be `DType::F32`, and the model's input shape is fixed at load time. `TensorMeta` carries up to `MAX_NDIM` (8) dimensions.

## Binary footprint

The design goal is to keep tract out of your binary. Approximate stripped sizes (they vary with platform, tract version, and your own code):

| Artifact | Links | Approx. size |
|----------|-------|--------------|
| `libzentract_abi.{so,dll,dylib}` | tract-onnx (the full engine) | ~16 MB |
| host footprint added by `zentract-api` | `libloading` + `thiserror` | ~350 KB |

The durable point isn't the exact figures — it's the ratio: tens of megabytes of engine compile once into a plugin you ship as a file, while every consumer rebuilds only a few hundred KB of loader.

## Platform support

CI covers Linux, macOS, and Windows on x86-64, plus Linux and macOS on ARM64. Windows on ARM64 is not built: tract's linear-algebra kernels ship ARM64 GAS assembly that the MSVC toolchain can't assemble. On any target tract doesn't support, link tract directly rather than through the plugin.

## License

Dual-licensed: [AGPL-3.0](https://github.com/imazen/zentract/blob/main/LICENSE-AGPL3) or [commercial](https://github.com/imazen/zentract/blob/main/LICENSE-COMMERCIAL).

I've maintained and developed open-source image server software — and the 40+
library ecosystem it depends on — full-time since 2011. Fifteen years of
continual maintenance, backwards compatibility, support, and the (very rare)
security patch. That kind of stability requires sustainable funding, and
dual-licensing is how we make it work without venture capital or rug-pulls.
Support sustainable and secure software; swap patch tuesday for patch leap-year.

[Our open-source products](https://www.imazen.io/open-source)

**Your options:**

- **Startup license** — $1 if your company has under $1M revenue and fewer
  than 5 employees. [Get a key →](https://www.imazen.io/pricing)
- **Commercial subscription** — Governed by the Imazen Site-wide Subscription
  License v1.1 or later. Apache 2.0-like terms, no source-sharing requirement.
  Sliding scale by company size.
  [Pricing & 60-day free trial →](https://www.imazen.io/pricing)
- **AGPL v3** — Free and open. Share your source if you distribute.

See [LICENSE-COMMERCIAL](https://github.com/imazen/zentract/blob/main/LICENSE-COMMERCIAL) for details.

## Image tech I maintain

| | |
|:--|:--|
| **Codecs** ¹ | [zenjpeg] · [zenpng] · [zenwebp] · [zengif] · [zenavif] · [zenjxl] · [zenjxl-decoder] · [jxl-encoder] · [zenbitmaps] · [heic] · [zentiff] · [zenpdf] · [zensvg] · [zenjp2] · [zenraw] · [ultrahdr] |
| Codec internals | [zenrav1e] · [rav1d-safe] · [zenravif] · [zenavif-parse] · [zenavif-serialize] |
| Compression | [zenflate] · [zenzop] · [zenzstd] |
| Processing | [zenresize] · [zenquant] · [zenblend] · [zenfilters] · [zensally] · [zentone] |
| Pixels & color | [zenpixels] · [zenpixels-convert] · [linear-srgb] · [garb] · [zenyuv] |
| Pipeline & framework | [zenpipe] · [zencodec] · [zencodecs] · [zenlayout] · [zennode] · [zenwasm] · **zentract** |
| Metrics | [zensim] · [fast-ssim2] · [butteraugli] · [zenmetrics] · [resamplescope-rs] |
| Pickers & ML | [zenanalyze] · [zenpredict] · [zenpicker] · [zenanalyze-api] |
| Test corpora | [codec-corpus] · [imazen-26] |
| Products | [Imageflow] image engine ([.NET][imageflow-dotnet] · [Node][imageflow-node] · [Go][imageflow-go]) · [Imageflow Server] · [ImageResizer] (C#) |

<sub>¹ pure-Rust, `#![forbid(unsafe_code)]` codecs, as of 2026</sub>

### General Rust awesomeness

[zenbench] · [archmage] · [magetypes] · [enough] · [whereat] · [cargo-copter] · [zenutils]

[Open source](https://www.imazen.io/open-source) · [@imazen](https://github.com/imazen) · [@lilith](https://github.com/lilith) · [lib.rs/~lilith](https://lib.rs/~lilith)

[zenjpeg]: https://github.com/imazen/zenjpeg
[zenpng]: https://github.com/imazen/zenpng
[zenwebp]: https://github.com/imazen/zenwebp
[zengif]: https://github.com/imazen/zengif
[zenavif]: https://github.com/imazen/zenavif
[zenjxl]: https://github.com/imazen/zenjxl
[zenjxl-decoder]: https://github.com/imazen/zenjxl-decoder
[jxl-encoder]: https://github.com/imazen/jxl-encoder
[zenbitmaps]: https://github.com/imazen/zenbitmaps
[heic]: https://github.com/imazen/heic
[zentiff]: https://github.com/imazen/zenextras
[zenpdf]: https://github.com/imazen/zenextras
[zensvg]: https://github.com/imazen/zenextras
[zenjp2]: https://github.com/imazen/zenextras
[zenraw]: https://github.com/imazen/zenraw
[ultrahdr]: https://github.com/imazen/ultrahdr
[zenrav1e]: https://github.com/imazen/zenrav1e
[rav1d-safe]: https://github.com/imazen/rav1d-safe
[zenravif]: https://github.com/imazen/cavif-rs
[zenavif-parse]: https://github.com/imazen/zenavif
[zenavif-serialize]: https://github.com/imazen/zenavif
[zenflate]: https://github.com/imazen/zenflate
[zenzop]: https://github.com/imazen/zenzop
[zenzstd]: https://github.com/imazen/zenzstd
[zenresize]: https://github.com/imazen/zenresize
[zenquant]: https://github.com/imazen/zenquant
[zenblend]: https://github.com/imazen/zenblend
[zenfilters]: https://github.com/imazen/zenpipe
[zensally]: https://github.com/imazen/zensally
[zentone]: https://github.com/imazen/zentone
[zenpixels]: https://github.com/imazen/zenpixels
[zenpixels-convert]: https://github.com/imazen/zenpixels
[linear-srgb]: https://github.com/imazen/linear-srgb
[garb]: https://github.com/imazen/garb
[zenyuv]: https://github.com/imazen/zenjpeg
[zenpipe]: https://github.com/imazen/zenpipe
[zencodec]: https://github.com/imazen/zencodec
[zencodecs]: https://github.com/imazen/zenpipe
[zenlayout]: https://github.com/imazen/zenpipe
[zennode]: https://github.com/imazen/zennode
[zenwasm]: https://github.com/imazen/zenwasm
[zensim]: https://github.com/imazen/zensim
[fast-ssim2]: https://github.com/imazen/fast-ssim2
[butteraugli]: https://github.com/imazen/butteraugli
[zenmetrics]: https://github.com/imazen/zenmetrics
[resamplescope-rs]: https://github.com/imazen/resamplescope-rs
[zenanalyze]: https://github.com/imazen/zenanalyze
[zenpredict]: https://github.com/imazen/zenanalyze
[zenpicker]: https://github.com/imazen/zenanalyze
[zenanalyze-api]: https://github.com/imazen/zenanalyze
[codec-corpus]: https://github.com/imazen/codec-corpus
[imazen-26]: https://github.com/imazen/imazen-26
[zenbench]: https://github.com/imazen/zenbench
[archmage]: https://github.com/imazen/archmage
[magetypes]: https://github.com/imazen/archmage
[enough]: https://github.com/imazen/enough
[whereat]: https://github.com/lilith/whereat
[cargo-copter]: https://github.com/imazen/cargo-copter
[zenutils]: https://github.com/imazen/zenutils
[Imageflow]: https://github.com/imazen/imageflow
[Imageflow Server]: https://github.com/imazen/imageflow-dotnet-server
[ImageResizer]: https://github.com/imazen/resizer
[imageflow-dotnet]: https://github.com/imazen/imageflow-dotnet
[imageflow-node]: https://github.com/imazen/imageflow-node
[imageflow-go]: https://github.com/imazen/imageflow-go
