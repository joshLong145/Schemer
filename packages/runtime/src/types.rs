//! Type definitions for heap objects
//!
//! This module defines the memory layout of heap-allocated Scheme objects.

use crate::gc::{alloc, scm_incref, RefCount, Value};
use crate::tags::{self, get_pointer, make_pointer, TAG_BOX, TAG_CLOSURE, TAG_PAIR, TAG_STRING};

/// Pair (cons cell) layout
#[repr(C)]
pub struct Pair {
    /// Reference count
    pub refcount: RefCount,
    /// Object type marker (unused for pairs, could be used for GC)
    pub objtype: u64,
    /// Car (first element)
    pub car: Value,
    /// Cdr (second element)  
    pub cdr: Value,
}

/// Closure layout
#[repr(C)]
pub struct Closure {
    /// Reference count
    pub refcount: RefCount,
    /// Object type marker
    pub objtype: u64,
    /// Function pointer
    pub func: *const (),
    /// Number of captured variables
    pub ncaptures: u64,
    /// Captured variables (flexible array member)
    pub captures: [Value; 0],
}

/// String layout
#[repr(C)]
pub struct ScmString {
    /// Reference count
    pub refcount: RefCount,
    /// Object type marker
    pub objtype: u64,
    /// Length in bytes
    pub length: u64,
    /// UTF-8 data (flexible array member)
    pub data: [u8; 0],
}

/// Vector layout
#[repr(C)]
pub struct Vector {
    /// Reference count
    pub refcount: RefCount,
    /// Object type marker
    pub objtype: u64,
    /// Number of elements
    pub length: u64,
    /// Elements (flexible array member)
    pub data: [Value; 0],
}

/// Box (mutable cell) layout
#[repr(C)]
pub struct Box {
    /// Reference count
    pub refcount: RefCount,
    /// Object type marker
    pub objtype: u64,
    /// Contained value
    pub value: Value,
}

/// Symbol table (for interning)
static mut SYMBOL_TABLE: Option<*mut SymbolTable> = None;

/// Simple symbol table using a vector of strings
struct SymbolTable {
    symbols: Vec<String>,
}

/// Initialize the symbol table
pub fn init_symbols() {
    unsafe {
        let table = std::boxed::Box::new(SymbolTable {
            symbols: Vec::new(),
        });
        SYMBOL_TABLE = Some(std::boxed::Box::into_raw(table));
    }
}

/// Clean up the symbol table
pub fn cleanup_symbols() {
    unsafe {
        if let Some(ptr) = SYMBOL_TABLE.take() {
            let _ = std::boxed::Box::from_raw(ptr);
        }
    }
}

// =============================================================================
// Allocation functions (called from compiled code)
// =============================================================================

/// Allocate a pair
#[no_mangle]
pub extern "C" fn scm_alloc_pair() -> Value {
    let ptr = alloc(tags::PAIR_SIZE);
    make_pointer(ptr as u64, TAG_PAIR)
}

/// Allocate a closure
#[no_mangle]
pub extern "C" fn scm_alloc_closure(func: *const (), ncaptures: u64) -> Value {
    let size = tags::CLOSURE_CAPTURES_OFFSET + (ncaptures as usize * 8);
    let ptr = alloc(size) as *mut Closure;
    unsafe {
        (*ptr).func = func;
        (*ptr).ncaptures = ncaptures;
        // Initialize captures to void
        let captures = (*ptr).captures.as_mut_ptr();
        for i in 0..ncaptures as usize {
            *captures.add(i) = tags::VALUE_VOID;
        }
    }
    make_pointer(ptr as u64, TAG_CLOSURE)
}

/// Allocate a string from a C string pointer and length
#[no_mangle]
pub extern "C" fn scm_alloc_string(data: *const u8, len: u64) -> Value {
    let size = tags::STRING_DATA_OFFSET + len as usize;
    let ptr = alloc(size) as *mut ScmString;
    unsafe {
        (*ptr).length = len;
        let dest = (*ptr).data.as_mut_ptr();
        std::ptr::copy_nonoverlapping(data, dest, len as usize);
    }
    make_pointer(ptr as u64, TAG_STRING)
}

/// Allocate a box
#[no_mangle]
pub extern "C" fn scm_alloc_box() -> Value {
    let ptr = alloc(tags::BOX_SIZE) as *mut Box;
    unsafe {
        (*ptr).value = tags::VALUE_VOID;
    }
    make_pointer(ptr as u64, TAG_BOX)
}

// =============================================================================
// Cons/car/cdr
// =============================================================================

/// Create a pair (cons)
#[no_mangle]
pub extern "C" fn scm_cons(car: Value, cdr: Value) -> Value {
    let pair_val = scm_alloc_pair();
    let ptr = get_pointer(pair_val) as *mut Pair;
    unsafe {
        (*ptr).car = car;
        (*ptr).cdr = cdr;
        // Increment refcounts of contained values
        scm_incref(car);
        scm_incref(cdr);
    }
    pair_val
}

/// Get car of a pair
#[no_mangle]
pub extern "C" fn scm_car(pair: Value) -> Value {
    // TODO: Type check
    let ptr = get_pointer(pair) as *const Pair;
    unsafe { (*ptr).car }
}

/// Get cdr of a pair
#[no_mangle]
pub extern "C" fn scm_cdr(pair: Value) -> Value {
    let ptr = get_pointer(pair) as *const Pair;
    unsafe { (*ptr).cdr }
}

/// Set car of a pair
#[no_mangle]
pub extern "C" fn scm_set_car(pair: Value, value: Value) -> Value {
    let ptr = get_pointer(pair) as *mut Pair;
    unsafe {
        let old = (*ptr).car;
        (*ptr).car = value;
        scm_incref(value);
        crate::gc::scm_decref(old);
    }
    tags::VALUE_VOID
}

/// Set cdr of a pair
#[no_mangle]
pub extern "C" fn scm_set_cdr(pair: Value, value: Value) -> Value {
    let ptr = get_pointer(pair) as *mut Pair;
    unsafe {
        let old = (*ptr).cdr;
        (*ptr).cdr = value;
        scm_incref(value);
        crate::gc::scm_decref(old);
    }
    tags::VALUE_VOID
}

// =============================================================================
// Closure operations
// =============================================================================

/// Get function pointer from closure
#[no_mangle]
pub extern "C" fn scm_closure_func(closure: Value) -> *const () {
    let ptr = get_pointer(closure) as *const Closure;
    unsafe { (*ptr).func }
}

/// Get captured value from closure
#[no_mangle]
pub extern "C" fn scm_closure_ref(closure: Value, index: u64) -> Value {
    let ptr = get_pointer(closure) as *const Closure;
    unsafe {
        let captures = (*ptr).captures.as_ptr();
        *captures.add(index as usize)
    }
}

/// Set captured value in closure
#[no_mangle]
pub extern "C" fn scm_closure_set(closure: Value, index: u64, value: Value) {
    let ptr = get_pointer(closure) as *mut Closure;
    unsafe {
        let captures = (*ptr).captures.as_mut_ptr();
        let old = *captures.add(index as usize);
        *captures.add(index as usize) = value;
        scm_incref(value);
        crate::gc::scm_decref(old);
    }
}

// =============================================================================
// Box operations
// =============================================================================

/// Get value from box
#[no_mangle]
pub extern "C" fn scm_box_ref(scm_box: Value) -> Value {
    let ptr = get_pointer(scm_box) as *const Box;
    unsafe { (*ptr).value }
}

/// Set value in box
#[no_mangle]
pub extern "C" fn scm_box_set(scm_box: Value, value: Value) -> Value {
    let ptr = get_pointer(scm_box) as *mut Box;
    unsafe {
        let old = (*ptr).value;
        (*ptr).value = value;
        scm_incref(value);
        crate::gc::scm_decref(old);
    }
    tags::VALUE_VOID
}

// =============================================================================
// Symbol interning
// =============================================================================

/// Intern a symbol (returns existing or creates new)
#[no_mangle]
pub extern "C" fn scm_intern_symbol(name: *const u8, len: u64) -> Value {
    unsafe {
        let table = SYMBOL_TABLE.expect("Symbol table not initialized");
        let name_str =
            std::str::from_utf8_unchecked(std::slice::from_raw_parts(name, len as usize));

        // Search for existing symbol
        for (i, sym) in (*table).symbols.iter().enumerate() {
            if sym == name_str {
                return make_symbol(i as u64);
            }
        }

        // Create new symbol
        let idx = (*table).symbols.len();
        (*table).symbols.push(String::from(name_str));
        make_symbol(idx as u64)
    }
}

/// Create a symbol value from an index
fn make_symbol(index: u64) -> Value {
    (index << tags::PAYLOAD_SHIFT) | tags::TAG_SYMBOL
}

/// Convert symbol to string
#[no_mangle]
pub extern "C" fn scm_symbol_to_string(symbol: Value) -> Value {
    let index = (symbol >> tags::PAYLOAD_SHIFT) as usize;
    unsafe {
        let table = SYMBOL_TABLE.expect("Symbol table not initialized");
        let symbols = &(*table).symbols;
        let name = &symbols[index];
        scm_alloc_string(name.as_ptr(), name.len() as u64)
    }
}
