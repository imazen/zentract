// deny rather than forbid: this crate wraps C ABI calls via libloading
#![deny(unsafe_code)]

pub use zentract_types::{ABI_VERSION, DType, ErrorCode, TensorMeta};

use libloading::Library;
use std::path::Path;

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("failed to load plugin library: {0}")]
    LoadLibrary(#[from] libloading::Error),
    #[error("ABI version mismatch: host expects {expected}, plugin has {actual}")]
    AbiMismatch { expected: u32, actual: u32 },
    #[error("model load failed (error code {0})")]
    ModelLoad(i64),
    #[error("inference failed (error code {0})")]
    Inference(i32),
    #[error("invalid handle")]
    InvalidHandle,
}

/// Inference output: copied data + shape metadata.
pub struct InferOutput {
    pub data: Vec<f32>,
    pub meta: TensorMeta,
}

// Function pointer types matching the zentract-abi exports.
type FnAbiVersion = unsafe extern "C" fn() -> u32;
type FnLoad = unsafe extern "C" fn(*const u8, usize, *const TensorMeta) -> i64;
type FnInfer = unsafe extern "C" fn(
    i64,
    *const f32,
    usize,
    u32,
    *mut *const f32,
    *mut usize,
    *mut TensorMeta,
) -> i32;
type FnOutputCount = unsafe extern "C" fn(i64) -> i32;
type FnFree = unsafe extern "C" fn(i64);

/// A loaded zentract plugin. Holds the dylib open.
pub struct InferenceEngine {
    _lib: Library,
    fn_load: FnLoad,
    fn_infer: FnInfer,
    fn_output_count: FnOutputCount,
    fn_free: FnFree,
}

// SAFETY: The underlying Library and function pointers are Send+Sync
// because the plugin uses a global Mutex internally.
// The function pointers are plain C functions with no thread-local state.
#[allow(unsafe_code)]
unsafe impl Send for InferenceEngine {}
#[allow(unsafe_code)]
unsafe impl Sync for InferenceEngine {}

impl InferenceEngine {
    /// Load the zentract plugin from a shared library path.
    #[allow(unsafe_code)]
    pub fn load(path: impl AsRef<Path>) -> Result<Self, Error> {
        let lib = unsafe { Library::new(path.as_ref()) }?;

        let fn_abi_version: FnAbiVersion =
            unsafe { *lib.get::<FnAbiVersion>(b"zentract_abi_version\0")? };
        let actual = unsafe { fn_abi_version() };
        if actual != ABI_VERSION {
            return Err(Error::AbiMismatch {
                expected: ABI_VERSION,
                actual,
            });
        }

        let fn_load: FnLoad = unsafe { *lib.get::<FnLoad>(b"zentract_load\0")? };
        let fn_infer: FnInfer = unsafe { *lib.get::<FnInfer>(b"zentract_infer\0")? };
        let fn_output_count: FnOutputCount =
            unsafe { *lib.get::<FnOutputCount>(b"zentract_output_count\0")? };
        let fn_free: FnFree = unsafe { *lib.get::<FnFree>(b"zentract_free\0")? };

        Ok(Self {
            _lib: lib,
            fn_load,
            fn_infer,
            fn_output_count,
            fn_free,
        })
    }

    /// Load an ONNX model from raw bytes with the given input shape.
    #[allow(unsafe_code)]
    pub fn load_onnx(
        &self,
        onnx_bytes: &[u8],
        input: TensorMeta,
    ) -> Result<ModelHandle<'_>, Error> {
        let handle =
            unsafe { (self.fn_load)(onnx_bytes.as_ptr(), onnx_bytes.len(), &input as *const _) };
        if handle < 0 {
            return Err(Error::ModelLoad(handle));
        }
        Ok(ModelHandle {
            engine: self,
            handle,
        })
    }

    /// Run inference on a raw model handle (from [`ModelHandle::into_raw`]).
    #[allow(unsafe_code)]
    pub fn infer_raw(
        &self,
        handle: i64,
        input: &[f32],
        output_index: u32,
    ) -> Result<InferOutput, Error> {
        let mut out_data: *const f32 = std::ptr::null();
        let mut out_len: usize = 0;
        let mut out_meta = TensorMeta::f32_shape(&[]);

        let rc = unsafe {
            (self.fn_infer)(
                handle,
                input.as_ptr(),
                input.len(),
                output_index,
                &mut out_data as *mut _,
                &mut out_len as *mut _,
                &mut out_meta as *mut _,
            )
        };

        if rc != 0 {
            return Err(Error::Inference(rc));
        }

        let data = unsafe { std::slice::from_raw_parts(out_data, out_len) }.to_vec();
        Ok(InferOutput {
            data,
            meta: out_meta,
        })
    }

    /// Free a raw model handle (from [`ModelHandle::into_raw`]).
    #[allow(unsafe_code)]
    pub fn free_raw(&self, handle: i64) {
        unsafe { (self.fn_free)(handle) };
    }
}

/// A loaded model. Freed on drop.
pub struct ModelHandle<'e> {
    engine: &'e InferenceEngine,
    handle: i64,
}

impl<'e> ModelHandle<'e> {
    /// Consume the handle, returning its raw ID without freeing the model.
    ///
    /// The model remains loaded in the plugin. Use
    /// [`InferenceEngine::infer_raw`] to run inference, and
    /// [`InferenceEngine::free_raw`] to free it when done.
    pub fn into_raw(self) -> i64 {
        let id = self.handle;
        std::mem::forget(self);
        id
    }

    /// Run inference. Returns a copy of the output tensor data.
    #[allow(unsafe_code)]
    pub fn infer(&self, input: &[f32], output_index: u32) -> Result<InferOutput, Error> {
        let mut out_data: *const f32 = std::ptr::null();
        let mut out_len: usize = 0;
        let mut out_meta = TensorMeta::f32_shape(&[]);

        let rc = unsafe {
            (self.engine.fn_infer)(
                self.handle,
                input.as_ptr(),
                input.len(),
                output_index,
                &mut out_data as *mut _,
                &mut out_len as *mut _,
                &mut out_meta as *mut _,
            )
        };

        if rc != 0 {
            return Err(Error::Inference(rc));
        }

        // Copy data out so caller doesn't depend on plugin lifetime
        let data = unsafe { std::slice::from_raw_parts(out_data, out_len) }.to_vec();

        Ok(InferOutput {
            data,
            meta: out_meta,
        })
    }

    /// Number of outputs the model produces.
    #[allow(unsafe_code)]
    pub fn output_count(&self) -> Result<u32, Error> {
        let rc = unsafe { (self.engine.fn_output_count)(self.handle) };
        if rc < 0 {
            return Err(Error::InvalidHandle);
        }
        Ok(rc as u32)
    }
}

#[allow(unsafe_code)]
impl Drop for ModelHandle<'_> {
    fn drop(&mut self) {
        unsafe { (self.engine.fn_free)(self.handle) };
    }
}
