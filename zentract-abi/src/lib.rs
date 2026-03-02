// No forbid(unsafe_code) — this is an FFI boundary crate.

use std::cell::RefCell;
use std::io::Cursor;

use tract_onnx::prelude::*;
use zentract_types::*;

struct LoadedModel {
    runnable: TypedRunnableModel<TypedModel>,
    input_meta: TensorMeta,
    output_count: u32,
    /// Holds last inference outputs so the caller can read output data
    /// via the returned pointer. Valid until next infer() or free().
    last_outputs: Option<TVec<TValue>>,
}

// TValue contains Rc (not Send), so use thread_local instead of Mutex.
thread_local! {
    static MODELS: RefCell<Vec<Option<LoadedModel>>> = const { RefCell::new(Vec::new()) };
}

#[unsafe(no_mangle)]
pub extern "C" fn zentract_abi_version() -> u32 {
    ABI_VERSION
}

/// Load an ONNX model from raw bytes with a fixed input shape.
/// Returns a handle (>= 0) on success, or a negative error code.
///
/// # Safety
/// `onnx_bytes` must point to `onnx_len` valid bytes.
/// `input_meta` must point to a valid TensorMeta.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn zentract_load(
    onnx_bytes: *const u8,
    onnx_len: usize,
    input_meta: *const TensorMeta,
) -> i64 {
    if onnx_bytes.is_null() || input_meta.is_null() {
        return ErrorCode::InvalidModel as i64;
    }

    let bytes = unsafe { std::slice::from_raw_parts(onnx_bytes, onnx_len) };
    let meta = unsafe { *input_meta };

    if meta.dtype != DType::F32 {
        return ErrorCode::InvalidModel as i64;
    }

    let shape: Vec<usize> = meta.shape[..meta.ndim as usize]
        .iter()
        .map(|&d| d as usize)
        .collect();

    let runnable = match load_onnx(bytes, &shape) {
        Ok(m) => m,
        Err(_) => return ErrorCode::InvalidModel as i64,
    };

    let output_count = runnable
        .model()
        .output_outlets()
        .map(|o| o.len() as u32)
        .unwrap_or(1);

    let loaded = LoadedModel {
        runnable,
        input_meta: meta,
        output_count,
        last_outputs: None,
    };

    MODELS.with(|models| {
        let mut models = models.borrow_mut();
        for (i, slot) in models.iter_mut().enumerate() {
            if slot.is_none() {
                *slot = Some(loaded);
                return i as i64;
            }
        }
        let id = models.len();
        models.push(Some(loaded));
        id as i64
    })
}

fn load_onnx(
    bytes: &[u8],
    shape: &[usize],
) -> Result<TypedRunnableModel<TypedModel>, Box<dyn std::error::Error>> {
    let model = tract_onnx::onnx()
        .model_for_read(&mut Cursor::new(bytes))?
        .with_input_fact(0, InferenceFact::dt_shape(DatumType::F32, shape))?
        .into_optimized()?
        .into_runnable()?;
    Ok(model)
}

/// Run inference on a loaded model.
///
/// The output pointer is valid until the next `zentract_infer` or
/// `zentract_free` call on the same handle.
///
/// # Safety
/// All pointers must be valid. `input` must point to `input_len` f32 values.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn zentract_infer(
    handle: i64,
    input: *const f32,
    input_len: usize,
    output_index: u32,
    out_data: *mut *const f32,
    out_len: *mut usize,
    out_meta: *mut TensorMeta,
) -> i32 {
    if input.is_null() || out_data.is_null() || out_len.is_null() || out_meta.is_null() {
        return ErrorCode::ShapeMismatch as i32;
    }

    let input_slice = unsafe { std::slice::from_raw_parts(input, input_len) };

    MODELS.with(|models| {
        let mut models = models.borrow_mut();
        let Some(Some(model)) = models.get_mut(handle as usize) else {
            return ErrorCode::InvalidHandle as i32;
        };

        let shape: Vec<usize> = model.input_meta.shape[..model.input_meta.ndim as usize]
            .iter()
            .map(|&d| d as usize)
            .collect();

        let expected_len: usize = shape.iter().product();
        if input_len != expected_len {
            return ErrorCode::ShapeMismatch as i32;
        }

        let tensor = match Tensor::from_shape(&shape, input_slice) {
            Ok(t) => t,
            Err(_) => return ErrorCode::InferenceFailed as i32,
        };

        let outputs = match model.runnable.run(tvec!(tensor.into())) {
            Ok(o) => o,
            Err(_) => return ErrorCode::InferenceFailed as i32,
        };

        let idx = output_index as usize;
        if idx >= outputs.len() {
            return ErrorCode::ShapeMismatch as i32;
        }

        // Capture shape before storing
        let out_shape: Vec<u64> = outputs[idx].shape().iter().map(|&d| d as u64).collect();

        // Store outputs to keep data alive
        model.last_outputs = Some(outputs);

        // Get pointer from stored data (the TValue data doesn't move)
        let stored = model.last_outputs.as_ref().unwrap();
        let slice = match stored[idx].as_slice::<f32>() {
            Ok(s) => s,
            Err(_) => return ErrorCode::InferenceFailed as i32,
        };

        unsafe {
            *out_data = slice.as_ptr();
            *out_len = slice.len();
            *out_meta = TensorMeta::f32_shape(&out_shape);
        }

        ErrorCode::Ok as i32
    })
}

/// Return the number of outputs for a loaded model.
#[unsafe(no_mangle)]
pub extern "C" fn zentract_output_count(handle: i64) -> i32 {
    MODELS.with(|models| {
        let models = models.borrow();
        match models.get(handle as usize) {
            Some(Some(model)) => model.output_count as i32,
            _ => ErrorCode::InvalidHandle as i32,
        }
    })
}

/// Free a loaded model and its cached outputs.
#[unsafe(no_mangle)]
pub extern "C" fn zentract_free(handle: i64) {
    MODELS.with(|models| {
        let mut models = models.borrow_mut();
        if let Some(slot) = models.get_mut(handle as usize) {
            *slot = None;
        }
    });
}
