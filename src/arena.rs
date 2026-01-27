//! Thread-local arena for fast bump allocation

use std::ptr;

#[repr(C)]
pub(crate) struct Arena {
    pub start: *mut u8,
    pub offset: usize,
    pub end: usize,
}

impl Arena {
    pub const fn new() -> Self {
        Self {
            start: ptr::null_mut(),
            offset: 0,
            end: 0,
        }
    }
}

#[thread_local]
pub(crate) static mut ARENA: Arena = Arena::new();

/// Try to allocate aligned memory from the thread-local arena
#[inline]
pub(crate) unsafe fn try_alloc_aligned(arena: *mut Arena, alignment: usize, size: usize) -> Option<*mut u8> {
    unsafe {
        let base = (*arena).start as usize + (*arena).offset;
        let data_addr = (base + 8 + alignment - 1) & !(alignment - 1);
        let header_addr = data_addr - 8;
        let new_offset = data_addr + size - (*arena).start as usize;

        if new_offset <= (*arena).end {
            (*arena).offset = new_offset;
            ptr::write(header_addr as *mut u64, size as u64);
            Some(data_addr as *mut u8)
        } else {
            None
        }
    }
}
