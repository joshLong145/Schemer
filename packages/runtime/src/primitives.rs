//! Primitive type predicates
//!
//! These functions check the type of Scheme values.

use crate::gc::Value;
use crate::tags::{self, get_tag, VALUE_FALSE, VALUE_TRUE};

/// Check if value is a pair
#[no_mangle]
pub extern "C" fn scm_is_pair(value: Value) -> Value {
    if get_tag(value) == tags::TAG_PAIR {
        VALUE_TRUE
    } else {
        VALUE_FALSE
    }
}

/// Check if value is a number (fixnum)
#[no_mangle]
pub extern "C" fn scm_is_number(value: Value) -> Value {
    if get_tag(value) == tags::TAG_FIXNUM {
        VALUE_TRUE
    } else {
        VALUE_FALSE
    }
}

/// Check if value is a boolean
#[no_mangle]
pub extern "C" fn scm_is_bool(value: Value) -> Value {
    if value == VALUE_TRUE || value == VALUE_FALSE {
        VALUE_TRUE
    } else {
        VALUE_FALSE
    }
}

/// Check if value is a symbol
#[no_mangle]
pub extern "C" fn scm_is_symbol(value: Value) -> Value {
    if get_tag(value) == tags::TAG_SYMBOL {
        VALUE_TRUE
    } else {
        VALUE_FALSE
    }
}

/// Check if value is a string
#[no_mangle]
pub extern "C" fn scm_is_string(value: Value) -> Value {
    if get_tag(value) == tags::TAG_STRING {
        VALUE_TRUE
    } else {
        VALUE_FALSE
    }
}

/// Check if value is a procedure (closure)
#[no_mangle]
pub extern "C" fn scm_is_procedure(value: Value) -> Value {
    if get_tag(value) == tags::TAG_CLOSURE {
        VALUE_TRUE
    } else {
        VALUE_FALSE
    }
}

/// Check if value is a character
#[no_mangle]
pub extern "C" fn scm_is_char(value: Value) -> Value {
    if get_tag(value) == tags::TAG_IMMEDIATE {
        let subtype = (value >> tags::IMMEDIATE_SUBTYPE_SHIFT) & 0b11111;
        if subtype == tags::IMMEDIATE_CHAR {
            return VALUE_TRUE;
        }
    }
    VALUE_FALSE
}

/// Check if value is null (empty list)
#[no_mangle]
pub extern "C" fn scm_is_null(value: Value) -> Value {
    if value == tags::VALUE_NIL {
        VALUE_TRUE
    } else {
        VALUE_FALSE
    }
}

/// Pointer equality (eq?)
#[no_mangle]
pub extern "C" fn scm_eq(a: Value, b: Value) -> Value {
    if a == b {
        VALUE_TRUE
    } else {
        VALUE_FALSE
    }
}

/// Equivalence (eqv?)
/// Same as eq? for most types, but compares numbers by value
#[no_mangle]
pub extern "C" fn scm_eqv(a: Value, b: Value) -> Value {
    // For fixnums, strings, and other types, eq? works
    // (Scheme numbers are already normalized as fixnums)
    scm_eq(a, b)
}

// =============================================================================
// Arithmetic primitives (for runtime calls, not inline)
// =============================================================================

/// Add two fixnums
#[no_mangle]
pub extern "C" fn scm_add(a: Value, b: Value) -> Value {
    let a_num = tags::get_fixnum(a);
    let b_num = tags::get_fixnum(b);
    tags::make_fixnum(a_num.wrapping_add(b_num))
}

/// Subtract two fixnums
#[no_mangle]
pub extern "C" fn scm_sub(a: Value, b: Value) -> Value {
    let a_num = tags::get_fixnum(a);
    let b_num = tags::get_fixnum(b);
    tags::make_fixnum(a_num.wrapping_sub(b_num))
}

/// Multiply two fixnums
#[no_mangle]
pub extern "C" fn scm_mul(a: Value, b: Value) -> Value {
    let a_num = tags::get_fixnum(a);
    let b_num = tags::get_fixnum(b);
    tags::make_fixnum(a_num.wrapping_mul(b_num))
}

/// Divide two fixnums
#[no_mangle]
pub extern "C" fn scm_div(a: Value, b: Value) -> Value {
    let a_num = tags::get_fixnum(a);
    let b_num = tags::get_fixnum(b);
    if b_num == 0 {
        crate::exceptions::scm_raise(tags::VALUE_FALSE); // TODO: proper error
    }
    tags::make_fixnum(a_num / b_num)
}

/// Modulo of two fixnums
#[no_mangle]
pub extern "C" fn scm_mod(a: Value, b: Value) -> Value {
    let a_num = tags::get_fixnum(a);
    let b_num = tags::get_fixnum(b);
    if b_num == 0 {
        crate::exceptions::scm_raise(tags::VALUE_FALSE);
    }
    tags::make_fixnum(a_num % b_num)
}

/// Numeric less than
#[no_mangle]
pub extern "C" fn scm_lt(a: Value, b: Value) -> Value {
    if tags::get_fixnum(a) < tags::get_fixnum(b) {
        VALUE_TRUE
    } else {
        VALUE_FALSE
    }
}

/// Numeric greater than
#[no_mangle]
pub extern "C" fn scm_gt(a: Value, b: Value) -> Value {
    if tags::get_fixnum(a) > tags::get_fixnum(b) {
        VALUE_TRUE
    } else {
        VALUE_FALSE
    }
}

/// Numeric less than or equal
#[no_mangle]
pub extern "C" fn scm_le(a: Value, b: Value) -> Value {
    if tags::get_fixnum(a) <= tags::get_fixnum(b) {
        VALUE_TRUE
    } else {
        VALUE_FALSE
    }
}

/// Numeric greater than or equal
#[no_mangle]
pub extern "C" fn scm_ge(a: Value, b: Value) -> Value {
    if tags::get_fixnum(a) >= tags::get_fixnum(b) {
        VALUE_TRUE
    } else {
        VALUE_FALSE
    }
}

/// Numeric equality
#[no_mangle]
pub extern "C" fn scm_num_eq(a: Value, b: Value) -> Value {
    if tags::get_fixnum(a) == tags::get_fixnum(b) {
        VALUE_TRUE
    } else {
        VALUE_FALSE
    }
}

/// Logical not
#[no_mangle]
pub extern "C" fn scm_not(value: Value) -> Value {
    if value == VALUE_FALSE {
        VALUE_TRUE
    } else {
        VALUE_FALSE
    }
}

// =============================================================================
// Vector operations
// =============================================================================

/// Check if value is a vector
#[no_mangle]
pub extern "C" fn scm_is_vector(value: Value) -> Value {
    if get_tag(value) == tags::TAG_VECTOR {
        VALUE_TRUE
    } else {
        VALUE_FALSE
    }
}

/// Allocate a vector
#[no_mangle]
pub extern "C" fn scm_make_vector(length: Value, fill: Value) -> Value {
    let len = tags::get_fixnum(length) as usize;
    let size = tags::VECTOR_DATA_OFFSET + len * 8;
    let ptr = crate::gc::alloc(size) as *mut crate::types::Vector;
    unsafe {
        (*ptr).length = len as u64;
        let data = (*ptr).data.as_mut_ptr();
        for i in 0..len {
            *data.add(i) = fill;
            crate::gc::scm_incref(fill);
        }
    }
    tags::make_pointer(ptr as u64, tags::TAG_VECTOR)
}

/// Get vector length
#[no_mangle]
pub extern "C" fn scm_vector_length(vec: Value) -> Value {
    let ptr = tags::get_pointer(vec) as *const crate::types::Vector;
    unsafe { tags::make_fixnum((*ptr).length as i64) }
}

/// Get vector element
#[no_mangle]
pub extern "C" fn scm_vector_ref(vec: Value, index: Value) -> Value {
    let ptr = tags::get_pointer(vec) as *const crate::types::Vector;
    let idx = tags::get_fixnum(index) as usize;
    unsafe {
        let data = (*ptr).data.as_ptr();
        *data.add(idx)
    }
}

/// Set vector element
#[no_mangle]
pub extern "C" fn scm_vector_set(vec: Value, index: Value, value: Value) -> Value {
    let ptr = tags::get_pointer(vec) as *mut crate::types::Vector;
    let idx = tags::get_fixnum(index) as usize;
    unsafe {
        let data = (*ptr).data.as_mut_ptr();
        let old = *data.add(idx);
        *data.add(idx) = value;
        crate::gc::scm_incref(value);
        crate::gc::scm_decref(old);
    }
    tags::VALUE_VOID
}

// =============================================================================
// String operations
// =============================================================================

/// Get string length
#[no_mangle]
pub extern "C" fn scm_string_length(s: Value) -> Value {
    let ptr = tags::get_pointer(s) as *const crate::types::ScmString;
    unsafe { tags::make_fixnum((*ptr).length as i64) }
}

/// Get character at index
#[no_mangle]
pub extern "C" fn scm_string_ref(s: Value, index: Value) -> Value {
    let ptr = tags::get_pointer(s) as *const crate::types::ScmString;
    let idx = tags::get_fixnum(index) as usize;
    unsafe {
        let data = (*ptr).data.as_ptr();
        let byte = *data.add(idx);
        tags::make_char(byte as char)
    }
}

/// Append two strings
#[no_mangle]
pub extern "C" fn scm_string_append(a: Value, b: Value) -> Value {
    let ptr_a = tags::get_pointer(a) as *const crate::types::ScmString;
    let ptr_b = tags::get_pointer(b) as *const crate::types::ScmString;
    unsafe {
        let len_a = (*ptr_a).length as usize;
        let len_b = (*ptr_b).length as usize;
        let total = len_a + len_b;

        let size = tags::STRING_DATA_OFFSET + total;
        let new_ptr = crate::gc::alloc(size) as *mut crate::types::ScmString;
        (*new_ptr).length = total as u64;

        let dest = (*new_ptr).data.as_mut_ptr();
        std::ptr::copy_nonoverlapping((*ptr_a).data.as_ptr(), dest, len_a);
        std::ptr::copy_nonoverlapping((*ptr_b).data.as_ptr(), dest.add(len_a), len_b);

        tags::make_pointer(new_ptr as u64, tags::TAG_STRING)
    }
}

// =============================================================================
// List utilities
// =============================================================================

/// Get list length
#[no_mangle]
pub extern "C" fn scm_length(list: Value) -> Value {
    let mut count = 0i64;
    let mut current = list;
    while current != tags::VALUE_NIL {
        count += 1;
        current = crate::types::scm_cdr(current);
    }
    tags::make_fixnum(count)
}

/// Reverse a list
#[no_mangle]
pub extern "C" fn scm_reverse(list: Value) -> Value {
    let mut result = tags::VALUE_NIL;
    let mut current = list;
    while current != tags::VALUE_NIL {
        let car = crate::types::scm_car(current);
        result = crate::types::scm_cons(car, result);
        current = crate::types::scm_cdr(current);
    }
    result
}

/// Append two lists
#[no_mangle]
pub extern "C" fn scm_append(a: Value, b: Value) -> Value {
    if a == tags::VALUE_NIL {
        return b;
    }
    let car = crate::types::scm_car(a);
    let cdr = crate::types::scm_cdr(a);
    crate::types::scm_cons(car, scm_append(cdr, b))
}

/// List ref (nth element)
#[no_mangle]
pub extern "C" fn scm_list_ref(list: Value, index: Value) -> Value {
    let mut idx = tags::get_fixnum(index);
    let mut current = list;
    while idx > 0 && current != tags::VALUE_NIL {
        current = crate::types::scm_cdr(current);
        idx -= 1;
    }
    if current == tags::VALUE_NIL {
        crate::exceptions::scm_raise(tags::VALUE_FALSE); // index out of bounds
    }
    crate::types::scm_car(current)
}

/// Member - find element in list using equal?
#[no_mangle]
pub extern "C" fn scm_member(obj: Value, list: Value) -> Value {
    let mut current = list;
    while current != tags::VALUE_NIL {
        if scm_equal(obj, crate::types::scm_car(current)) == tags::VALUE_TRUE {
            return current;
        }
        current = crate::types::scm_cdr(current);
    }
    tags::VALUE_FALSE
}

/// Memq - find element using eq?
#[no_mangle]
pub extern "C" fn scm_memq(obj: Value, list: Value) -> Value {
    let mut current = list;
    while current != tags::VALUE_NIL {
        if obj == crate::types::scm_car(current) {
            return current;
        }
        current = crate::types::scm_cdr(current);
    }
    tags::VALUE_FALSE
}

/// Assoc - find pair by key using equal?
#[no_mangle]
pub extern "C" fn scm_assoc(key: Value, alist: Value) -> Value {
    let mut current = alist;
    while current != tags::VALUE_NIL {
        let pair = crate::types::scm_car(current);
        if get_tag(pair) == tags::TAG_PAIR
            && scm_equal(key, crate::types::scm_car(pair)) == tags::VALUE_TRUE
        {
            return pair;
        }
        current = crate::types::scm_cdr(current);
    }
    tags::VALUE_FALSE
}

/// Assq - find pair by key using eq?
#[no_mangle]
pub extern "C" fn scm_assq(key: Value, alist: Value) -> Value {
    let mut current = alist;
    while current != tags::VALUE_NIL {
        let pair = crate::types::scm_car(current);
        if get_tag(pair) == tags::TAG_PAIR && key == crate::types::scm_car(pair) {
            return pair;
        }
        current = crate::types::scm_cdr(current);
    }
    tags::VALUE_FALSE
}

// =============================================================================
// Deep equality (equal?)
// =============================================================================

/// Deep structural equality
#[no_mangle]
pub extern "C" fn scm_equal(a: Value, b: Value) -> Value {
    if a == b {
        return VALUE_TRUE;
    }

    let tag_a = get_tag(a);
    let tag_b = get_tag(b);

    if tag_a != tag_b {
        return VALUE_FALSE;
    }

    match tag_a {
        tags::TAG_PAIR => {
            if scm_equal(crate::types::scm_car(a), crate::types::scm_car(b)) == VALUE_FALSE {
                return VALUE_FALSE;
            }
            scm_equal(crate::types::scm_cdr(a), crate::types::scm_cdr(b))
        }
        tags::TAG_STRING => {
            let ptr_a = tags::get_pointer(a) as *const crate::types::ScmString;
            let ptr_b = tags::get_pointer(b) as *const crate::types::ScmString;
            unsafe {
                let len_a = (*ptr_a).length;
                let len_b = (*ptr_b).length;
                if len_a != len_b {
                    return VALUE_FALSE;
                }
                let data_a = std::slice::from_raw_parts((*ptr_a).data.as_ptr(), len_a as usize);
                let data_b = std::slice::from_raw_parts((*ptr_b).data.as_ptr(), len_b as usize);
                if data_a == data_b {
                    VALUE_TRUE
                } else {
                    VALUE_FALSE
                }
            }
        }
        tags::TAG_VECTOR => {
            let ptr_a = tags::get_pointer(a) as *const crate::types::Vector;
            let ptr_b = tags::get_pointer(b) as *const crate::types::Vector;
            unsafe {
                let len_a = (*ptr_a).length;
                let len_b = (*ptr_b).length;
                if len_a != len_b {
                    return VALUE_FALSE;
                }
                for i in 0..len_a as usize {
                    let elem_a = *(*ptr_a).data.as_ptr().add(i);
                    let elem_b = *(*ptr_b).data.as_ptr().add(i);
                    if scm_equal(elem_a, elem_b) == VALUE_FALSE {
                        return VALUE_FALSE;
                    }
                }
                VALUE_TRUE
            }
        }
        _ => VALUE_FALSE,
    }
}

// =============================================================================
// Character operations
// =============================================================================

/// Character to integer
#[no_mangle]
pub extern "C" fn scm_char_to_integer(c: Value) -> Value {
    let ch = tags::get_char(c);
    tags::make_fixnum(ch as i64)
}

/// Integer to character
#[no_mangle]
pub extern "C" fn scm_integer_to_char(n: Value) -> Value {
    let code = tags::get_fixnum(n) as u32;
    tags::make_char(char::from_u32(code).unwrap_or('\0'))
}

/// Character equality
#[no_mangle]
pub extern "C" fn scm_char_eq(a: Value, b: Value) -> Value {
    if a == b {
        VALUE_TRUE
    } else {
        VALUE_FALSE
    }
}

/// Character less than
#[no_mangle]
pub extern "C" fn scm_char_lt(a: Value, b: Value) -> Value {
    if tags::get_char(a) < tags::get_char(b) {
        VALUE_TRUE
    } else {
        VALUE_FALSE
    }
}

// =============================================================================
// Numeric utilities
// =============================================================================

/// Absolute value
#[no_mangle]
pub extern "C" fn scm_abs(n: Value) -> Value {
    let num = tags::get_fixnum(n);
    tags::make_fixnum(num.abs())
}

/// Minimum of two numbers
#[no_mangle]
pub extern "C" fn scm_min(a: Value, b: Value) -> Value {
    let na = tags::get_fixnum(a);
    let nb = tags::get_fixnum(b);
    tags::make_fixnum(na.min(nb))
}

/// Maximum of two numbers
#[no_mangle]
pub extern "C" fn scm_max(a: Value, b: Value) -> Value {
    let na = tags::get_fixnum(a);
    let nb = tags::get_fixnum(b);
    tags::make_fixnum(na.max(nb))
}

/// Check if number is zero
#[no_mangle]
pub extern "C" fn scm_zero_p(n: Value) -> Value {
    if tags::get_fixnum(n) == 0 {
        VALUE_TRUE
    } else {
        VALUE_FALSE
    }
}

/// Check if number is positive
#[no_mangle]
pub extern "C" fn scm_positive_p(n: Value) -> Value {
    if tags::get_fixnum(n) > 0 {
        VALUE_TRUE
    } else {
        VALUE_FALSE
    }
}

/// Check if number is negative
#[no_mangle]
pub extern "C" fn scm_negative_p(n: Value) -> Value {
    if tags::get_fixnum(n) < 0 {
        VALUE_TRUE
    } else {
        VALUE_FALSE
    }
}

/// Check if number is odd
#[no_mangle]
pub extern "C" fn scm_odd_p(n: Value) -> Value {
    if tags::get_fixnum(n) % 2 != 0 {
        VALUE_TRUE
    } else {
        VALUE_FALSE
    }
}

/// Check if number is even
#[no_mangle]
pub extern "C" fn scm_even_p(n: Value) -> Value {
    if tags::get_fixnum(n) % 2 == 0 {
        VALUE_TRUE
    } else {
        VALUE_FALSE
    }
}

// =============================================================================
// Apply and call utilities
// =============================================================================

/// Apply function to list of arguments
#[no_mangle]
pub extern "C" fn scm_apply(func: Value, args: Value) -> Value {
    // Count args and build array
    let mut argc = 0usize;
    let mut current = args;
    while current != tags::VALUE_NIL {
        argc += 1;
        current = crate::types::scm_cdr(current);
    }

    // Get function pointer
    let func_ptr = crate::types::scm_closure_func(func);

    // Build args array on stack (for small arities)
    let mut arg_array = [0u64; 8];
    current = args;
    for slot in arg_array.iter_mut().take(argc.min(8)) {
        *slot = crate::types::scm_car(current);
        current = crate::types::scm_cdr(current);
    }

    // Call through trampoline
    // SAFETY: func_ptr is extracted from a valid closure and matches the expected arity
    unsafe { crate::trampoline::call_with_args_array(func, func_ptr, &arg_array[..argc]) }
}
