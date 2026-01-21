//! Exception handling using setjmp/longjmp
//!
//! This module provides exception handling for Scheme programs using
//! setjmp/longjmp for non-local control flow.

use crate::gc::Value;
use crate::tags::VALUE_VOID;
use backtrace::Backtrace;
use std::mem::MaybeUninit;

/// Maximum depth of exception handler stack
const MAX_HANDLERS: usize = 64;

/// Platform-specific jmp_buf size (conservative estimate)
/// On most platforms: x86_64 Linux ~200 bytes, macOS ~148 bytes, aarch64 ~256 bytes
const JMP_BUF_SIZE: usize = 256;

/// Our own jmp_buf type - a byte array large enough for any platform
#[repr(C, align(16))]
#[derive(Copy, Clone)]
pub struct JmpBuf {
    buf: [u8; JMP_BUF_SIZE],
}

impl Default for JmpBuf {
    fn default() -> Self {
        Self {
            buf: [0; JMP_BUF_SIZE],
        }
    }
}

#[allow(dead_code)]
extern "C" {
    /// setjmp returns 0 on direct call, non-zero when returning via longjmp
    fn setjmp(buf: *mut JmpBuf) -> i32;
    /// longjmp jumps back to the setjmp call point
    fn longjmp(buf: *mut JmpBuf, val: i32) -> !;
}

/// Exception handler stack
struct HandlerStack {
    /// Current handler depth
    depth: usize,
    /// Jump buffers for each handler level
    handlers: [MaybeUninit<JmpBuf>; MAX_HANDLERS],
    /// Current exception value (if any)
    current_exception: Value,
    /// Backtrace captured when exception was raised
    current_backtrace: Option<Backtrace>,
    /// Call stack depth at each handler level (from backtrace)
    handler_stack_depths: [usize; MAX_HANDLERS],
}

static mut HANDLERS: HandlerStack = HandlerStack {
    depth: 0,
    handlers: unsafe { MaybeUninit::uninit().assume_init() },
    current_exception: 0,
    current_backtrace: None,
    handler_stack_depths: [0; MAX_HANDLERS],
};

/// Initialize exception handling
pub fn init_handlers() {
    unsafe {
        HANDLERS.depth = 0;
        HANDLERS.current_exception = VALUE_VOID;
        HANDLERS.current_backtrace = None;
        // Initialize all handler slots
        for i in 0..MAX_HANDLERS {
            HANDLERS.handlers[i] = MaybeUninit::new(JmpBuf::default());
            HANDLERS.handler_stack_depths[i] = 0;
        }
    }
}

/// Get current call stack depth using backtrace
fn get_stack_depth() -> usize {
    let bt = Backtrace::new_unresolved();
    bt.frames().len()
}

/// Push an exception handler
/// Returns pointer to jmp_buf for use with setjmp
///
/// Usage from generated code:
/// ```c
/// JmpBuf* buf = scm_push_handler();
/// if (setjmp(buf) == 0) {
///     // normal execution
///     scm_pop_handler();
/// } else {
///     // exception was raised
///     Value exc = scm_current_exception();
/// }
/// ```
#[no_mangle]
pub extern "C" fn scm_push_handler() -> *mut JmpBuf {
    unsafe {
        if HANDLERS.depth >= MAX_HANDLERS {
            eprintln!("Exception handler stack overflow");
            std::process::abort();
        }
        let idx = HANDLERS.depth;
        // Record call stack depth at this handler level
        HANDLERS.handler_stack_depths[idx] = get_stack_depth();
        HANDLERS.depth += 1;
        HANDLERS.handlers[idx].as_mut_ptr()
    }
}

/// Pop an exception handler
#[no_mangle]
pub extern "C" fn scm_pop_handler() {
    unsafe {
        if HANDLERS.depth > 0 {
            HANDLERS.depth -= 1;
        }
    }
}

/// Raise an exception - performs longjmp to nearest handler
#[no_mangle]
pub extern "C" fn scm_raise(exception: Value) -> ! {
    unsafe {
        HANDLERS.current_exception = exception;
        // Capture backtrace at point of exception
        HANDLERS.current_backtrace = Some(Backtrace::new());

        if HANDLERS.depth > 0 {
            // Jump to the most recent handler
            let idx = HANDLERS.depth - 1;
            HANDLERS.depth = idx; // Pop the handler we're jumping to
            let buf = HANDLERS.handlers[idx].as_mut_ptr();
            longjmp(buf, 1);
        } else {
            // No handler - print and abort
            eprintln!("Unhandled exception:");
            crate::io::scm_display(exception);
            crate::io::scm_newline();
            // Print backtrace for debugging
            if let Some(ref bt) = HANDLERS.current_backtrace {
                eprintln!("Backtrace:\n{:?}", bt);
            }
            std::process::abort();
        }
    }
}

/// Get the current exception value
#[no_mangle]
pub extern "C" fn scm_current_exception() -> Value {
    unsafe { HANDLERS.current_exception }
}

/// Create an error object from a string
/// # Safety
/// `message` must point to at least `len` valid bytes.
#[no_mangle]
pub unsafe extern "C" fn scm_make_error(message: *const u8, len: u64) -> Value {
    crate::types::scm_alloc_string(message, len)
}

/// Check if we have an active exception handler
#[no_mangle]
pub extern "C" fn scm_has_handler() -> bool {
    unsafe { HANDLERS.depth > 0 }
}

/// Get current handler depth (for debugging)
#[no_mangle]
pub extern "C" fn scm_handler_depth() -> usize {
    unsafe { HANDLERS.depth }
}

/// Get the call stack depth at the current exception point
/// Returns 0 if no exception backtrace is available
#[no_mangle]
pub extern "C" fn scm_exception_stack_depth() -> usize {
    unsafe {
        (*std::ptr::addr_of!(HANDLERS))
            .current_backtrace
            .as_ref()
            .map(|bt| bt.frames().len())
            .unwrap_or(0)
    }
}

/// Get the call stack depth at a specific handler level
/// Returns 0 if the handler level is invalid
#[no_mangle]
pub extern "C" fn scm_handler_stack_depth(handler_level: usize) -> usize {
    unsafe {
        if handler_level < HANDLERS.depth {
            HANDLERS.handler_stack_depths[handler_level]
        } else {
            0
        }
    }
}

/// Get the number of frames unwound when the last exception was raised
/// (difference between exception point and handler it jumped to)
#[no_mangle]
pub extern "C" fn scm_frames_unwound() -> usize {
    unsafe {
        if let Some(ref bt) = HANDLERS.current_backtrace {
            let exception_depth = bt.frames().len();
            // The handler we jumped to is at HANDLERS.depth (after being decremented in scm_raise)
            if HANDLERS.depth < MAX_HANDLERS {
                let handler_depth = HANDLERS.handler_stack_depths[HANDLERS.depth];
                exception_depth.saturating_sub(handler_depth)
            } else {
                0
            }
        } else {
            0
        }
    }
}

/// Print the current exception backtrace to stderr
#[no_mangle]
pub extern "C" fn scm_print_backtrace() {
    unsafe {
        if let Some(ref bt) = HANDLERS.current_backtrace {
            eprintln!("{:?}", bt);
        } else {
            eprintln!("No backtrace available");
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_handler_push_pop() {
        init_handlers();
        assert_eq!(unsafe { HANDLERS.depth }, 0);

        let _ = scm_push_handler();
        assert_eq!(unsafe { HANDLERS.depth }, 1);

        let _ = scm_push_handler();
        assert_eq!(unsafe { HANDLERS.depth }, 2);

        scm_pop_handler();
        assert_eq!(unsafe { HANDLERS.depth }, 1);

        scm_pop_handler();
        assert_eq!(unsafe { HANDLERS.depth }, 0);
    }

    #[test]
    fn test_has_handler() {
        init_handlers();
        assert!(!scm_has_handler());

        let _ = scm_push_handler();
        assert!(scm_has_handler());

        scm_pop_handler();
        assert!(!scm_has_handler());
    }

    #[test]
    fn test_exception_with_setjmp() {
        init_handlers();

        let buf = scm_push_handler();
        let result = unsafe { setjmp(buf) };

        if result == 0 {
            // Normal path - simulate raising exception
            // We don't actually call scm_raise here as it would longjmp
            // Just verify the setup works
            scm_pop_handler();
            assert!(true);
        } else {
            // Would be exception path
            panic!("Unexpected longjmp");
        }
    }
}
