//! I/O operations
//!
//! Basic input/output for Scheme programs.

use crate::gc::Value;
use crate::tags::{self, get_fixnum, get_pointer, get_tag, VALUE_VOID};
use crate::types::ScmString;

/// Display a value to stdout
#[no_mangle]
pub extern "C" fn scm_display(value: Value) -> Value {
    display_value(value, false);
    VALUE_VOID
}

/// Write a value to stdout (with quotes for strings)
#[no_mangle]
pub extern "C" fn scm_write(value: Value) -> Value {
    display_value(value, true);
    VALUE_VOID
}

/// Print a newline
#[no_mangle]
pub extern "C" fn scm_newline() -> Value {
    unsafe {
        libc::putchar(b'\n' as i32);
    }
    VALUE_VOID
}

/// Internal display implementation
fn display_value(value: Value, write_mode: bool) {
    let tag = get_tag(value);

    match tag {
        tags::TAG_FIXNUM => {
            let n = get_fixnum(value);
            print_int(n);
        }
        tags::TAG_PAIR => {
            print_char('(');
            print_list(value, write_mode);
            print_char(')');
        }
        tags::TAG_CLOSURE => {
            print_str("#<procedure>");
        }
        tags::TAG_STRING => {
            let ptr = get_pointer(value) as *const ScmString;
            unsafe {
                let len = (*ptr).length as usize;
                let data = (*ptr).data.as_ptr();
                if write_mode {
                    print_char('"');
                }
                for i in 0..len {
                    let c = *data.add(i);
                    if write_mode && (c == b'"' || c == b'\\') {
                        print_char('\\');
                    }
                    libc::putchar(c as i32);
                }
                if write_mode {
                    print_char('"');
                }
            }
        }
        tags::TAG_SYMBOL => {
            // Get symbol name from table
            let sym_str = crate::types::scm_symbol_to_string(value);
            display_value(sym_str, false);
            // Decref the temporary string
            crate::gc::scm_decref(sym_str);
        }
        tags::TAG_VECTOR => {
            print_str("#(");
            // TODO: Print vector elements
            print_str("...)");
        }
        tags::TAG_BOX => {
            print_str("#<box>");
        }
        tags::TAG_IMMEDIATE => {
            if value == tags::VALUE_TRUE {
                print_str("#t");
            } else if value == tags::VALUE_FALSE {
                print_str("#f");
            } else if value == tags::VALUE_NIL {
                print_str("()");
            } else if value == tags::VALUE_VOID {
                // Don't print void
            } else if value == tags::VALUE_EOF {
                print_str("#<eof>");
            } else {
                // Character
                let subtype = (value >> tags::IMMEDIATE_SUBTYPE_SHIFT) & 0b11111;
                if subtype == tags::IMMEDIATE_CHAR {
                    let codepoint = (value >> 8) as u32;
                    if write_mode {
                        print_str("#\\");
                    }
                    if let Some(c) = char::from_u32(codepoint) {
                        print_char(c);
                    }
                } else {
                    print_str("#<unknown>");
                }
            }
        }
        _ => {
            print_str("#<unknown>");
        }
    }
}

/// Print a list (helper for display)
fn print_list(value: Value, write_mode: bool) {
    let ptr = get_pointer(value) as *const crate::types::Pair;
    unsafe {
        display_value((*ptr).car, write_mode);

        let cdr = (*ptr).cdr;
        if cdr == tags::VALUE_NIL {
            // End of proper list
        } else if get_tag(cdr) == tags::TAG_PAIR {
            // Continue list
            print_char(' ');
            print_list(cdr, write_mode);
        } else {
            // Improper list
            print_str(" . ");
            display_value(cdr, write_mode);
        }
    }
}

/// Print an integer
fn print_int(n: i64) {
    if n < 0 {
        print_char('-');
        print_uint((-n) as u64);
    } else {
        print_uint(n as u64);
    }
}

/// Print an unsigned integer
fn print_uint(mut n: u64) {
    if n == 0 {
        print_char('0');
        return;
    }

    let mut digits = [0u8; 20];
    let mut i = 0;

    while n > 0 {
        digits[i] = (n % 10) as u8 + b'0';
        n /= 10;
        i += 1;
    }

    while i > 0 {
        i -= 1;
        unsafe {
            libc::putchar(digits[i] as i32);
        }
    }
}

/// Print a single character
fn print_char(c: char) {
    let mut buf = [0u8; 4];
    let s = c.encode_utf8(&mut buf);
    for b in s.bytes() {
        unsafe {
            libc::putchar(b as i32);
        }
    }
}

/// Print a string literal
fn print_str(s: &str) {
    for b in s.bytes() {
        unsafe {
            libc::putchar(b as i32);
        }
    }
}
