//! Trampoline support for tail call optimization
//!
//! This module implements trampoline-based TCO, where tail calls return
//! a "thunk" that the trampoline loop evaluates.

use crate::gc::{alloc, scm_incref, Value};
use crate::tags::{self, get_pointer, get_tag, make_pointer, TAG_CLOSURE, VALUE_FALSE, VALUE_TRUE};

/// Thunk tag - we use a special tag in the immediate space
/// A thunk is represented as: TAG_IMMEDIATE | (THUNK_MARKER << 3) | (thunk_ptr << 8)
/// But for simplicity, we'll use a heap-allocated structure with a special marker
/// Object type marker for thunks
#[allow(dead_code)]
const OBJTYPE_THUNK: u64 = 0x5448554E4B0000; // "THUNK\0\0" as hex

/// Thunk layout - represents a suspended tail call
#[repr(C)]
pub struct Thunk {
    /// Reference count (unused for thunks, but keeps layout consistent)
    pub refcount: u64,
    /// Object type marker (OBJTYPE_THUNK)
    pub objtype: u64,
    /// Function to call
    pub func: Value,
    /// Arguments (as a list or vector)
    pub args: Value,
}

/// Marker value to distinguish thunks from regular closures
/// We use a specific bit pattern in the objtype field
pub const THUNK_MARKER: u64 = 0x544855_4E4B0000; // "THUNK\0\0" as hex

/// Create a thunk for a tail call
#[no_mangle]
pub extern "C" fn scm_make_thunk(func: Value, args: Value) -> Value {
    let ptr = alloc(core::mem::size_of::<Thunk>()) as *mut Thunk;
    unsafe {
        (*ptr).objtype = THUNK_MARKER;
        (*ptr).func = func;
        (*ptr).args = args;
        scm_incref(func);
        scm_incref(args);
    }
    // Use closure tag for thunks (they're callable)
    make_pointer(ptr as u64, TAG_CLOSURE)
}

/// Check if a value is a thunk
#[no_mangle]
pub extern "C" fn scm_is_thunk(value: Value) -> Value {
    if get_tag(value) != TAG_CLOSURE {
        return VALUE_FALSE;
    }
    let ptr = get_pointer(value) as *const Thunk;
    unsafe {
        if (*ptr).objtype == THUNK_MARKER {
            VALUE_TRUE
        } else {
            VALUE_FALSE
        }
    }
}

/// Check if a value is a thunk (returns bool for internal use)
#[inline]
pub fn is_thunk(value: Value) -> bool {
    if get_tag(value) != TAG_CLOSURE {
        return false;
    }
    let ptr = get_pointer(value) as *const Thunk;
    unsafe { (*ptr).objtype == THUNK_MARKER }
}

/// Get the function from a thunk
#[no_mangle]
pub extern "C" fn scm_thunk_func(thunk: Value) -> Value {
    let ptr = get_pointer(thunk) as *const Thunk;
    unsafe { (*ptr).func }
}

/// Get the arguments from a thunk
#[no_mangle]
pub extern "C" fn scm_thunk_args(thunk: Value) -> Value {
    let ptr = get_pointer(thunk) as *const Thunk;
    unsafe { (*ptr).args }
}

/// Trampoline loop - bounces until we get a non-thunk result
///
/// This is the main entry point for executing compiled Scheme code.
/// It calls the initial function and then keeps bouncing on thunks
/// until a final value is reached.
#[no_mangle]
pub extern "C" fn scm_trampoline(mut value: Value) -> Value {
    while is_thunk(value) {
        let func = scm_thunk_func(value);
        let args = scm_thunk_args(value);

        // Free the thunk (we've extracted its contents)
        crate::gc::scm_decref(value);

        // Call the function with args
        // The function pointer expects (closure, arg1, arg2, ...)
        // For now, we'll need to unpack the args list
        value = call_with_args(func, args);
    }
    value
}

/// Call a closure with a list of arguments
fn call_with_args(closure: Value, args: Value) -> Value {
    // Get the function pointer from the closure
    let func_ptr = crate::types::scm_closure_func(closure);

    // Count and collect arguments
    let mut arg_list = args;
    let mut arg_count = 0;
    let mut collected_args = [0u64; 16]; // Max 16 args for now

    while arg_list != tags::VALUE_NIL {
        if arg_count >= 16 {
            // Too many arguments - should raise an error
            break;
        }
        collected_args[arg_count] = crate::types::scm_car(arg_list);
        arg_list = crate::types::scm_cdr(arg_list);
        arg_count += 1;
    }

    // Call based on arity
    // This is ugly but necessary without varargs support
    unsafe {
        type Fn0 = extern "C" fn(Value) -> Value;
        type Fn1 = extern "C" fn(Value, Value) -> Value;
        type Fn2 = extern "C" fn(Value, Value, Value) -> Value;
        type Fn3 = extern "C" fn(Value, Value, Value, Value) -> Value;
        type Fn4 = extern "C" fn(Value, Value, Value, Value, Value) -> Value;

        match arg_count {
            0 => {
                let f: Fn0 = core::mem::transmute(func_ptr);
                f(closure)
            }
            1 => {
                let f: Fn1 = core::mem::transmute(func_ptr);
                f(closure, collected_args[0])
            }
            2 => {
                let f: Fn2 = core::mem::transmute(func_ptr);
                f(closure, collected_args[0], collected_args[1])
            }
            3 => {
                let f: Fn3 = core::mem::transmute(func_ptr);
                f(
                    closure,
                    collected_args[0],
                    collected_args[1],
                    collected_args[2],
                )
            }
            4 => {
                let f: Fn4 = core::mem::transmute(func_ptr);
                f(
                    closure,
                    collected_args[0],
                    collected_args[1],
                    collected_args[2],
                    collected_args[3],
                )
            }
            _ => {
                // TODO: Support more arguments or use a different calling convention
                tags::VALUE_VOID
            }
        }
    }
}

/// Call a closure with an array of arguments (public version for apply)
///
/// # Safety
/// `func_ptr` must be a valid function pointer with the correct signature for the argument count.
pub unsafe fn call_with_args_array(closure: Value, func_ptr: *const (), args: &[u64]) -> Value {
    {
        type Fn0 = extern "C" fn(Value) -> Value;
        type Fn1 = extern "C" fn(Value, Value) -> Value;
        type Fn2 = extern "C" fn(Value, Value, Value) -> Value;
        type Fn3 = extern "C" fn(Value, Value, Value, Value) -> Value;
        type Fn4 = extern "C" fn(Value, Value, Value, Value, Value) -> Value;
        type Fn5 = extern "C" fn(Value, Value, Value, Value, Value, Value) -> Value;
        type Fn6 = extern "C" fn(Value, Value, Value, Value, Value, Value, Value) -> Value;
        type Fn7 = extern "C" fn(Value, Value, Value, Value, Value, Value, Value, Value) -> Value;

        match args.len() {
            0 => {
                let f: Fn0 = std::mem::transmute(func_ptr);
                f(closure)
            }
            1 => {
                let f: Fn1 = std::mem::transmute(func_ptr);
                f(closure, args[0])
            }
            2 => {
                let f: Fn2 = std::mem::transmute(func_ptr);
                f(closure, args[0], args[1])
            }
            3 => {
                let f: Fn3 = std::mem::transmute(func_ptr);
                f(closure, args[0], args[1], args[2])
            }
            4 => {
                let f: Fn4 = std::mem::transmute(func_ptr);
                f(closure, args[0], args[1], args[2], args[3])
            }
            5 => {
                let f: Fn5 = std::mem::transmute(func_ptr);
                f(closure, args[0], args[1], args[2], args[3], args[4])
            }
            6 => {
                let f: Fn6 = std::mem::transmute(func_ptr);
                f(
                    closure, args[0], args[1], args[2], args[3], args[4], args[5],
                )
            }
            7 => {
                let f: Fn7 = std::mem::transmute(func_ptr);
                f(
                    closure, args[0], args[1], args[2], args[3], args[4], args[5], args[6],
                )
            }
            _ => crate::tags::VALUE_VOID, // TODO: error for too many args
        }
    }
}
