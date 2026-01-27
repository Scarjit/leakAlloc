//! Global heap management

use std::ptr;
use std::sync::atomic::{AtomicPtr, AtomicUsize, Ordering};

use crate::{CHUNK_SIZE, HEAP_SIZE};

pub(crate) static HEAP_START: AtomicPtr<u8> = AtomicPtr::new(ptr::null_mut());
pub(crate) static HEAP_OFFSET: AtomicUsize = AtomicUsize::new(0);

fn init_heap() -> *mut u8 {
    let ptr = unsafe {
        libc::mmap(
            ptr::null_mut(),
            HEAP_SIZE,
            libc::PROT_READ | libc::PROT_WRITE,
            libc::MAP_PRIVATE | libc::MAP_ANONYMOUS | libc::MAP_POPULATE,
            -1,
            0,
        )
    };

    if ptr == libc::MAP_FAILED {
        return ptr::null_mut();
    }

    ptr as *mut u8
}

pub(crate) fn get_heap() -> *mut u8 {
    let mut heap = HEAP_START.load(Ordering::Acquire);
    if heap.is_null() {
        let new_heap = init_heap();
        if new_heap.is_null() {
            return ptr::null_mut();
        }
        match HEAP_START.compare_exchange(
            ptr::null_mut(),
            new_heap,
            Ordering::AcqRel,
            Ordering::Acquire,
        ) {
            Ok(_) => heap = new_heap,
            Err(existing) => {
                // Another thread initialized first, unmap ours
                unsafe { libc::munmap(new_heap as *mut libc::c_void, HEAP_SIZE) };
                heap = existing;
            }
        }
    }
    heap
}

/// Fetch a new chunk from the global heap for thread-local use
pub(crate) fn fetch_chunk(min_size: usize) -> Option<(*mut u8, usize)> {
    let heap = get_heap();
    if heap.is_null() {
        return None;
    }

    let chunk_size = min_size.max(CHUNK_SIZE);
    let offset = HEAP_OFFSET.fetch_add(chunk_size, Ordering::Relaxed);
    if offset + chunk_size > HEAP_SIZE {
        return None;
    }

    Some((unsafe { heap.add(offset) }, chunk_size))
}

/// Allocate directly from global heap (for large allocations)
pub(crate) fn alloc_global(total: usize, size: usize) -> *mut u8 {
    let heap = get_heap();
    if heap.is_null() {
        return ptr::null_mut();
    }

    let offset = HEAP_OFFSET.fetch_add(total, Ordering::Relaxed);
    if offset + total > HEAP_SIZE {
        return ptr::null_mut();
    }

    unsafe {
        let block = heap.add(offset);
        ptr::write(block as *mut u64, size as u64);
        block.add(8)
    }
}

/// Allocate aligned memory directly from global heap
pub(crate) fn alloc_aligned_global(alignment: usize, size: usize) -> *mut u8 {
    let heap = get_heap();
    if heap.is_null() {
        return ptr::null_mut();
    }

    loop {
        let offset = HEAP_OFFSET.load(Ordering::Relaxed);
        let base = heap as usize + offset;

        // Find aligned position for data (after 8-byte header)
        let data_addr = (base + 8 + alignment - 1) & !(alignment - 1);
        let header_addr = data_addr - 8;
        let new_offset = data_addr + size - heap as usize;

        if new_offset > HEAP_SIZE {
            return ptr::null_mut();
        }

        if HEAP_OFFSET
            .compare_exchange_weak(offset, new_offset, Ordering::Relaxed, Ordering::Relaxed)
            .is_ok()
        {
            unsafe {
                ptr::write(header_addr as *mut u64, size as u64);
                return data_addr as *mut u8;
            }
        }
    }
}
