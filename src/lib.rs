#![feature(thread_local)]

mod alloc;
mod arena;
mod heap;

pub(crate) const HEAP_SIZE: usize = 1024 * 1024 * 1024 * 16; // 16GB
pub(crate) const CHUNK_SIZE: usize = 1024 * 1024; // 1MB per thread chunk

// Re-export public API
pub use alloc::{aligned_alloc, calloc, free, malloc, memalign, posix_memalign, realloc};
