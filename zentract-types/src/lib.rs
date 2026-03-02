#![no_std]
#![forbid(unsafe_code)]

/// ABI version. Bump on breaking changes to the FFI interface.
pub const ABI_VERSION: u32 = 1;

/// Maximum number of tensor dimensions.
pub const MAX_NDIM: usize = 8;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(C)]
pub enum DType {
    F32 = 0,
    F16 = 1,
    U8 = 2,
    I8 = 3,
}

/// Describes a tensor's shape and element type. Fixed-size, stack-allocated.
#[derive(Clone, Copy, Debug)]
#[repr(C)]
pub struct TensorMeta {
    pub dtype: DType,
    pub ndim: u32,
    pub shape: [u64; MAX_NDIM],
}

impl TensorMeta {
    /// Create an F32 tensor descriptor from a shape slice.
    pub fn f32_shape(shape: &[u64]) -> Self {
        let mut s = [0u64; MAX_NDIM];
        let ndim = shape.len().min(MAX_NDIM);
        s[..ndim].copy_from_slice(&shape[..ndim]);
        Self {
            dtype: DType::F32,
            ndim: ndim as u32,
            shape: s,
        }
    }

    /// Total number of elements in this tensor.
    pub fn num_elements(&self) -> u64 {
        self.shape[..self.ndim as usize].iter().product()
    }
}

/// Error codes returned across the FFI boundary.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(i32)]
pub enum ErrorCode {
    Ok = 0,
    InvalidModel = -1,
    ShapeMismatch = -2,
    InferenceFailed = -3,
    InvalidHandle = -4,
    AbiMismatch = -5,
}
