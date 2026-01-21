//! Garbage collection with reference counting and trial deletion
//!
//! This module implements reference counting with trial deletion for cycle
//! collection. The algorithm is based on the paper "A Trial Deletion Algorithm
//! for Concurrent Reference Counting" but simplified for single-threaded use.

use crate::tags::{
    self, get_pointer, get_tag, TAG_BOX, TAG_CLOSURE, TAG_PAIR, TAG_STRING, TAG_VECTOR,
};
use crate::types::{Box as ScmBox, Closure, Pair, Vector};

use std::sync::Mutex;

/// Static buffer of possible cycle roots
static ROOTS: Mutex<Vec<Value>> = Mutex::new(Vec::new());

/// Scheme value type (tagged 64-bit word)
pub type Value = u64;

/// Reference count type
pub type RefCount = u64;

/// Special refcount value indicating the object is in the roots set
pub const REFCOUNT_ROOT: RefCount = u64::MAX;

/// Color for trial deletion algorithm
#[repr(u8)]
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Color {
    /// In use, refcount > 0
    Black = 0,
    /// Possible root of garbage cycle
    Purple = 1,
    /// In trial deletion
    Gray = 2,
    /// Confirmed garbage
    White = 3,
}

/// Offset within objtype field where color is stored
const COLOR_OFFSET: usize = 0;

/// Get the color of a heap object
#[inline]
unsafe fn get_color(value: Value) -> Color {
    let ptr = get_pointer(value) as *mut u8;
    let objtype_ptr = ptr.add(tags::OBJTYPE_OFFSET) as *mut u64;
    let color_byte = (*objtype_ptr >> (COLOR_OFFSET * 8)) as u8;
    match color_byte {
        0 => Color::Black,
        1 => Color::Purple,
        2 => Color::Gray,
        3 => Color::White,
        _ => Color::Black, // Default to Black for unknown values
    }
}

/// Set the color of a heap object
#[inline]
unsafe fn set_color(value: Value, color: Color) {
    let ptr = get_pointer(value) as *mut u8;
    let objtype_ptr = ptr.add(tags::OBJTYPE_OFFSET) as *mut u64;
    let mask = !(0xFF_u64 << (COLOR_OFFSET * 8));
    let color_bits = (color as u64) << (COLOR_OFFSET * 8);
    *objtype_ptr = (*objtype_ptr & mask) | color_bits;
}

/// Get the reference count of a heap object (internal use)
#[inline]
unsafe fn get_rc(value: Value) -> RefCount {
    let ptr = get_pointer(value) as *mut RefCount;
    *ptr
}

/// Set the reference count of a heap object (internal use)
#[inline]
unsafe fn set_rc(value: Value, rc: RefCount) {
    let ptr = get_pointer(value) as *mut RefCount;
    *ptr = rc;
}

/// Call a function for each child of a heap object
unsafe fn for_each_child<F: FnMut(Value)>(value: Value, mut f: F) {
    let tag = get_tag(value);
    let ptr = get_pointer(value) as *mut u8;

    match tag {
        TAG_PAIR => {
            let pair = ptr as *mut Pair;
            f((*pair).car);
            f((*pair).cdr);
        }
        TAG_CLOSURE => {
            let closure = ptr as *mut Closure;
            let ncaptures = (*closure).ncaptures as usize;
            let captures = (*closure).captures.as_ptr();
            for i in 0..ncaptures {
                f(*captures.add(i));
            }
        }
        TAG_VECTOR => {
            let vector = ptr as *mut Vector;
            let len = (*vector).length as usize;
            let data = (*vector).data.as_ptr();
            for i in 0..len {
                f(*data.add(i));
            }
        }
        TAG_BOX => {
            let scm_box = ptr as *mut ScmBox;
            f((*scm_box).value);
        }
        TAG_STRING => {
            // Strings have no children
        }
        _ => {
            // Unknown tag - no children
        }
    }
}

/// Initialize the heap
pub fn init_heap() {
    // For now, we use the system allocator directly
    // In a production implementation, we might want a custom heap
}

/// Increment reference count for a value
#[no_mangle]
pub extern "C" fn scm_incref(value: Value) {
    if !is_heap_object(value) {
        return;
    }

    let ptr = get_pointer(value) as *mut RefCount;
    unsafe {
        let rc = *ptr;
        if rc != REFCOUNT_ROOT {
            *ptr = rc.saturating_add(1);
        }
    }
}

/// Decrement reference count for a value
#[no_mangle]
pub extern "C" fn scm_decref(value: Value) {
    if !is_heap_object(value) {
        return;
    }

    let ptr = get_pointer(value) as *mut RefCount;
    unsafe {
        let rc = *ptr;
        if rc == REFCOUNT_ROOT {
            return;
        }
        if rc == 0 {
            // Already at zero - double decref bug
            return;
        }
        let new_rc = rc - 1;
        *ptr = new_rc;

        if new_rc == 0 {
            // Object is garbage - free it
            free_object(value);
        } else {
            // Possible cycle root - mark for trial deletion
            mark_possible_root(value);
        }
    }
}

/// Check if a value is a heap-allocated object
#[inline]
fn is_heap_object(value: Value) -> bool {
    let tag = get_tag(value);
    matches!(
        tag,
        TAG_PAIR | TAG_CLOSURE | TAG_STRING | TAG_VECTOR | TAG_BOX
    )
}

/// Free a garbage object and decref its children
unsafe fn free_object(value: Value) {
    let tag = get_tag(value);
    let ptr = get_pointer(value) as *mut u8;

    match tag {
        TAG_PAIR => {
            let pair = ptr as *mut Pair;
            scm_decref((*pair).car);
            scm_decref((*pair).cdr);
            libc::free(ptr as *mut libc::c_void);
        }
        TAG_CLOSURE => {
            let closure = ptr as *mut Closure;
            let ncaptures = (*closure).ncaptures as usize;
            let captures = (*closure).captures.as_ptr();
            for i in 0..ncaptures {
                scm_decref(*captures.add(i));
            }
            libc::free(ptr as *mut libc::c_void);
        }
        TAG_STRING => {
            // Strings don't contain references
            libc::free(ptr as *mut libc::c_void);
        }
        TAG_VECTOR => {
            let vector = ptr as *mut Vector;
            let len = (*vector).length as usize;
            let data = (*vector).data.as_ptr();
            for i in 0..len {
                scm_decref(*data.add(i));
            }
            libc::free(ptr as *mut libc::c_void);
        }
        TAG_BOX => {
            let scm_box = ptr as *mut ScmBox;
            scm_decref((*scm_box).value);
            libc::free(ptr as *mut libc::c_void);
        }
        _ => {
            // Unknown tag - shouldn't happen
        }
    }
}

/// Mark a value as a possible cycle root
fn mark_possible_root(value: Value) {
    if !is_heap_object(value) {
        return;
    }

    unsafe {
        let color = get_color(value);
        if color != Color::Purple {
            set_color(value, Color::Purple);
            let mut roots = ROOTS.lock().unwrap();
            roots.push(value);
        }
    }
}

/// Collect all garbage (called at shutdown)
pub fn collect_all() {
    collect_cycles();
}

/// Run the trial deletion cycle collection algorithm
fn collect_cycles() {
    // Take ownership of the current roots
    let roots = {
        let mut roots_guard = ROOTS.lock().unwrap();
        std::mem::take(&mut *roots_guard)
    };

    if roots.is_empty() {
        return;
    }

    // Phase 1: Mark gray (trial decrement)
    // For each purple root, perform trial deletion
    for &root in &roots {
        unsafe {
            if is_heap_object(root) && get_color(root) == Color::Purple {
                mark_gray(root);
            }
        }
    }

    // Phase 2: Scan (determine if truly garbage)
    // Check if decremented nodes are actually garbage
    for &root in &roots {
        unsafe {
            if is_heap_object(root) {
                scan(root);
            }
        }
    }

    // Phase 3: Collect white (free garbage)
    // Free all confirmed garbage
    for &root in &roots {
        unsafe {
            if is_heap_object(root) {
                collect_white(root);
            }
        }
    }
}

/// Mark gray: trial decrement children recursively
unsafe fn mark_gray(value: Value) {
    if !is_heap_object(value) {
        return;
    }

    if get_color(value) != Color::Gray {
        set_color(value, Color::Gray);

        // Trial decrement all children
        for_each_child(value, |child| {
            if is_heap_object(child) {
                let rc = get_rc(child);
                if rc > 0 && rc != REFCOUNT_ROOT {
                    set_rc(child, rc - 1);
                }
                mark_gray(child);
            }
        });
    }
}

/// Scan: determine if a gray node is truly garbage
unsafe fn scan(value: Value) {
    if !is_heap_object(value) {
        return;
    }

    if get_color(value) == Color::Gray {
        let rc = get_rc(value);
        if rc > 0 {
            // Still has external references - restore
            scan_black(value);
        } else {
            // No external references - mark as garbage
            set_color(value, Color::White);
            for_each_child(value, |child| {
                scan(child);
            });
        }
    }
}

/// Scan black: restore decremented refcounts (not garbage)
unsafe fn scan_black(value: Value) {
    if !is_heap_object(value) {
        return;
    }

    set_color(value, Color::Black);

    // Restore refcounts of children
    for_each_child(value, |child| {
        if is_heap_object(child) {
            let rc = get_rc(child);
            if rc != REFCOUNT_ROOT {
                set_rc(child, rc + 1);
            }
            if get_color(child) != Color::Black {
                scan_black(child);
            }
        }
    });
}

/// Collect white: free confirmed garbage
unsafe fn collect_white(value: Value) {
    if !is_heap_object(value) {
        return;
    }

    if get_color(value) == Color::White {
        // Mark black to prevent double-free
        set_color(value, Color::Black);

        // Collect children first
        for_each_child(value, |child| {
            collect_white(child);
        });

        // Free this object
        let ptr = get_pointer(value) as *mut libc::c_void;
        libc::free(ptr);
    }
}

/// Allocate memory for a heap object
pub fn alloc(size: usize) -> *mut u8 {
    unsafe {
        let ptr = libc::malloc(size) as *mut u8;
        if ptr.is_null() {
            // Out of memory - in a real implementation we'd try to collect first
            libc::abort();
        }
        // Zero the header
        core::ptr::write_bytes(ptr, 0, tags::HEADER_SIZE);
        // Set initial refcount to 1
        *(ptr as *mut RefCount) = 1;
        ptr
    }
}

/// Get the reference count of a value (for debugging)
#[no_mangle]
pub extern "C" fn scm_refcount(value: Value) -> RefCount {
    if !is_heap_object(value) {
        return 0;
    }
    unsafe {
        let ptr = get_pointer(value) as *mut RefCount;
        *ptr
    }
}
