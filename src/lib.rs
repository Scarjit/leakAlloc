use std::mem::size_of;
use std::ptr;
use std::sync::atomic::{AtomicPtr, AtomicUsize, Ordering};

const HEAP_SIZE: usize = 1024 * 1024 * 1024 * 16; // 16GB

static HEAP_START: AtomicPtr<u8> = AtomicPtr::new(ptr::null_mut());
static HEAP_OFFSET: AtomicUsize = AtomicUsize::new(0);

fn init_heap() -> *mut u8 {
    let ptr = unsafe {
        libc::mmap(
            ptr::null_mut(),
            HEAP_SIZE,
            libc::PROT_READ | libc::PROT_WRITE,
            libc::MAP_PRIVATE | libc::MAP_ANONYMOUS,
            -1,
            0,
        )
    };

    if ptr == libc::MAP_FAILED {
        return ptr::null_mut();
    }

    ptr as *mut u8
}

fn get_heap() -> *mut u8 {
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

#[unsafe(no_mangle)]
pub extern "C" fn malloc(size: usize) -> *mut u8 {
    if size == 0 {
        return ptr::null_mut();
    }

    let heap = get_heap();
    if heap.is_null() {
        return ptr::null_mut();
    }

    // size + 8 for header, aligned to 8 bytes
    let total = (size + 8 + 7) & !7;

    let offset = HEAP_OFFSET.fetch_add(total, Ordering::Relaxed);
    if offset + total > HEAP_SIZE {
        return ptr::null_mut();
    }

    unsafe {
        let block = heap.add(offset);
        // Write size header
        ptr::write(block as *mut u64, size as u64);
        // Return pointer after header
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
    let new_ptr = malloc(size);

    if !new_ptr.is_null() {
        let copy_size = old_size.min(size);
        unsafe { ptr::copy_nonoverlapping(ptr, new_ptr, copy_size) };
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
