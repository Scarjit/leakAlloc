//! Public allocation API (malloc, free, calloc, realloc, memalign, etc.)

use std::mem::size_of;
use std::ptr;

use crate::arena::{try_alloc_aligned, LOCAL_END, LOCAL_OFFSET, LOCAL_START};
use crate::heap::{alloc_aligned_global, alloc_global, fetch_chunk};
use crate::CHUNK_SIZE;


#[unsafe(no_mangle)]
pub extern "C" fn malloc(size: usize) -> *mut u8 {
    if size == 0 {
        return ptr::null_mut();
    }

    // size + 8 for header, aligned to 8 bytes
    let total = (size + 8 + 7) & !7;

    // Large allocations go directly to global heap
    if total > CHUNK_SIZE / 2 {
        return alloc_global(total, size);
    }

    unsafe {
        let remaining = LOCAL_END.saturating_sub(LOCAL_OFFSET);

        // Need a new chunk?
        if remaining < total {
            match fetch_chunk(CHUNK_SIZE) {
                Some((start, chunk_size)) => {
                    LOCAL_START = start;
                    LOCAL_OFFSET = 0;
                    LOCAL_END = chunk_size;
                }
                None => return ptr::null_mut(),
            }
        }

        let block = LOCAL_START.add(LOCAL_OFFSET);
        ptr::write(block as *mut u64, size as u64);
        LOCAL_OFFSET += total;
        block.add(8)
    }
}


#[unsafe(no_mangle)]
pub extern "C" fn free(_ptr: *mut u8) {
    // Intentional no-op
}

#[unsafe(no_mangle)]
pub extern "C" fn calloc(nmemb: usize, size: usize) -> *mut u8 {
    let total = match nmemb.checked_mul(size) {
        Some(t) => t,
        None => return ptr::null_mut(),
    };

    let ptr = malloc(total);
    if !ptr.is_null() {
        unsafe { ptr::write_bytes(ptr, 0, total) };
    }
    ptr
}

#[unsafe(no_mangle)]
pub extern "C" fn realloc(ptr: *mut u8, size: usize) -> *mut u8 {
    if ptr.is_null() {
        return malloc(size);
    }
    if size == 0 {
        return ptr::null_mut();
    }

    let old_size = unsafe { ptr::read((ptr as *const u64).sub(1)) } as usize;

    // Shrinking? Just return same pointer
    if size <= old_size {
        return ptr;
    }

    // Check if this is the last allocation in thread-local arena (can extend)
    unsafe {
        let block_start = ptr.sub(8);
        let block_end_offset = block_start.offset_from(LOCAL_START) as usize + 8 + ((old_size + 7) & !7);

        if block_start >= LOCAL_START && block_end_offset == LOCAL_OFFSET {
            let additional = ((size + 7) & !7) - ((old_size + 7) & !7);
            if LOCAL_OFFSET + additional <= LOCAL_END {
                LOCAL_OFFSET += additional;
                ptr::write((ptr as *mut u64).sub(1), size as u64);
                return ptr;
            }
        }
    }

    // Fallback: allocate + copy
    let new_ptr = malloc(size);
    if !new_ptr.is_null() {
        unsafe { ptr::copy_nonoverlapping(ptr, new_ptr, old_size) };
    }
    new_ptr
}

#[unsafe(no_mangle)]
pub extern "C" fn memalign(alignment: usize, size: usize) -> *mut u8 {
    // Alignment must be power of 2 and >= 8
    if size == 0 || alignment == 0 || (alignment & (alignment - 1)) != 0 {
        return ptr::null_mut();
    }
    let alignment = alignment.max(8);

    // Worst case: we need (alignment - 1) bytes for padding + 8 for header + size
    let max_total = alignment - 1 + 8 + size;

    // Large or highly-aligned allocations go directly to global heap
    if max_total > CHUNK_SIZE / 2 {
        return alloc_aligned_global(alignment, size);
    }

    unsafe {
        // Try current arena first
        if let Some(ptr) = try_alloc_aligned(alignment, size) {
            return ptr;
        }

        // Need a new chunk
        match fetch_chunk(CHUNK_SIZE) {
            Some((start, chunk_size)) => {
                LOCAL_START = start;
                LOCAL_OFFSET = 0;
                LOCAL_END = chunk_size;
            }
            None => return ptr::null_mut(),
        }

        try_alloc_aligned(alignment, size).unwrap_or(ptr::null_mut())
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn aligned_alloc(alignment: usize, size: usize) -> *mut u8 {
    memalign(alignment, size)
}

#[unsafe(no_mangle)]
pub extern "C" fn posix_memalign(memptr: *mut *mut u8, alignment: usize, size: usize) -> i32 {
    if memptr.is_null() || alignment < size_of::<usize>() || (alignment & (alignment - 1)) != 0 {
        return libc::EINVAL;
    }
    let ptr = memalign(alignment, size);
    if ptr.is_null() {
        return libc::ENOMEM;
    }
    unsafe { *memptr = ptr };
    0
}
