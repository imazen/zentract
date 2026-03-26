# zentract

Rust-to-Rust dynamic library boundary for [tract](https://github.com/sonos/tract) ONNX inference. Keeps the heavy tract dependency (267 crates, 16 MB) out of your binary; loads it at runtime via `dlopen`.

## Why

tract-onnx is great for pure-Rust ML inference, but it pulls in 7 sub-crates and dominates compile time for any project that uses it. If all you need is "f32 tensor in, f32 tensor out," there's no reason to statically link it.

zentract puts tract behind a cdylib with a C ABI. Your application depends only on `zentract-api` (14 crates, 350 KB stripped) and loads the plugin at runtime.

## Crates

| Crate | Type | Description |
|-------|------|-------------|
| `zentract-types` | lib (`no_std`) | Shared `#[repr(C)]` types: `TensorMeta`, `DType`, `ErrorCode` |
| `zentract-abi` | cdylib | Plugin that links tract-onnx, exports `extern "C"` functions |
| `zentract-api` | lib | Host wrapper using `libloading` — no tract dependency |

## Usage

```rust
use zentract_api::{InferenceEngine, TensorMeta};

// Load the plugin (once, at startup)
let engine = InferenceEngine::load("libzentract_abi.so")?;

// Load an ONNX model with a fixed input shape
let onnx_bytes = std::fs::read("model.onnx")?;
let input_shape = TensorMeta::f32_shape(&[1, 3, 320, 320]);
let model = engine.load_onnx(&onnx_bytes, input_shape)?;

// Run inference
let input: Vec<f32> = preprocess_image(/* ... */);
let output = model.infer(&input, 0)?; // output_index = 0
let scores: &[f32] = &output.data;
```

## Binary sizes (stripped)

| Artifact | Size |
|----------|------|
| `libzentract_abi.so` (tract inside) | 16 MB |
| Host binary (libloading only) | 350 KB |

## ABI contract

The plugin exports five `extern "C"` functions:

- `zentract_abi_version() -> u32`
- `zentract_load(onnx_bytes, len, input_meta) -> handle`
- `zentract_infer(handle, input, len, output_index, out_ptr, out_len, out_meta) -> error`
- `zentract_output_count(handle) -> count`
- `zentract_free(handle)`

Output pointers reference data inside the plugin. Valid until the next `infer` or `free` call on the same handle. The host copies out what it needs.

## Building

Both plugin and host are in the same workspace, built simultaneously:

```sh
cargo build --release
```

This produces `target/release/libzentract_abi.so` (the plugin) and makes `zentract-api` available for downstream crates.

## License

Dual-licensed: [AGPL-3.0](LICENSE-AGPL3) or [commercial](LICENSE-COMMERCIAL).

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

See [LICENSE-COMMERCIAL](LICENSE-COMMERCIAL) for details.
