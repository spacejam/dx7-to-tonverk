
//! Core constants for the FM synthesis engine

/// Log base 2 of the block size for processing
pub const LG_N: usize = 6;

/// Block size for audio processing (64 samples)
pub const N: usize = 1 << LG_N;

/// Memory barrier for synchronization
#[inline]
pub fn synth_memory_barrier() {
    #[cfg(feature = "std")]
    {
        use std::sync::atomic::{compiler_fence, Ordering};
        compiler_fence(Ordering::SeqCst);
    }
}

/// Quantized envelope rate conversion
#[inline]
pub const fn qer(n: i32, b: i32) -> f32 {
    (n as f32) / ((1 << b) as f32)
}

/// Utility functions for min/max (Rust std provides these, but keeping for consistency)
#[inline]
pub fn min<T: PartialOrd>(a: T, b: T) -> T {
    if a < b { a } else { b }
}

#[inline]
pub fn max<T: PartialOrd>(a: T, b: T) -> T {
    if a > b { a } else { b }
}