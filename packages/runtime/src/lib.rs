//! Schemer Native Runtime Library
//!
//! This library provides the runtime support for native-compiled Scheme programs.
//! It includes:
//! - Memory allocation and garbage collection (reference counting with trial deletion)
//! - Type predicates and conversions
//! - Primitive operations (cons, car, cdr, etc.)
//! - I/O operations
//! - Exception handling (setjmp/longjmp based)
//! - Trampoline support for tail call optimization
//!
//! All public functions use the `scm_` prefix and C calling convention for
//! interoperability with QBE-generated code.

pub mod exceptions;
pub mod gc;
pub mod io;
pub mod primitives;
pub mod tags;
pub mod trampoline;
pub mod types;

/// Initialize the runtime
/// Must be called before any other runtime functions
#[no_mangle]
pub extern "C" fn scm_init() {
    // Initialize the heap
    gc::init_heap();
    // Initialize the symbol table
    types::init_symbols();
    // Initialize exception handling
    exceptions::init_handlers();
}

/// Shutdown the runtime
/// Should be called before program exit for clean shutdown
#[no_mangle]
pub extern "C" fn scm_shutdown() {
    // Run final garbage collection
    gc::collect_all();
    // Clean up resources
    types::cleanup_symbols();
}
