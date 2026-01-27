//! Thread-local arena for fast bump allocation

use std::ptr;

#[thread_local]
pub(crate) static mut LOCAL_START: *mut u8 = ptr::null_mut();
#[thread_local]
pub(crate) static mut LOCAL_OFFSET: usize = 0;
#[thread_local]
pub(crate) static mut LOCAL_END: usize = 0;

/// Try to allocate aligned memory from the thread-local arena
#[inline]
pub(crate) unsafe fn try_alloc_aligned(alignment: usize, size: usize) -> Option<*mut u8> {
    unsafe {
        let base = LOCAL_START as usize + LOCAL_OFFSET;
        let data_addr = (base + 8 + alignment - 1) & !(alignment - 1);
        let header_addr = data_addr - 8;
        let new_offset = data_addr + size - LOCAL_START as usize;

        if new_offset <= LOCAL_END {
            LOCAL_OFFSET = new_offset;
            ptr::write(header_addr as *mut u64, size as u64);
            Some(data_addr as *mut u8)
        } else {
            None
        }
    }
}
