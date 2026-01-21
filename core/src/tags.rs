//! Shared tag definitions for the Schemer runtime
//!
//! This module defines the tagged pointer representation used for Scheme values.
//! Values are represented as 64-bit words with a 3-bit tag in the low bits:
//!
//! ```text
//! 63                                               3  2  1  0
//! +------------------------------------------------+--+--+--+
//! |                   payload (61 bits)            | tag  |
//! +------------------------------------------------+--+--+--+
//! ```
//!
//! The tag determines how to interpret the payload.

/// Tag bits occupy the low 3 bits of a tagged value
pub const TAG_BITS: u32 = 3;

/// Mask to extract the tag from a tagged value
pub const TAG_MASK: u64 = 0b111;

/// Mask to extract the payload (shift right by TAG_BITS)
pub const PAYLOAD_SHIFT: u32 = TAG_BITS;

// =============================================================================
// Primary Tags (3 bits)
// =============================================================================

/// Fixnum: 61-bit signed integer stored directly in payload
/// Value = payload as i61 (sign-extended)
pub const TAG_FIXNUM: u64 = 0b000;

/// Pair/cons cell: pointer to heap-allocated pair
/// Payload = pointer >> 3 (pairs are 16-byte aligned)
pub const TAG_PAIR: u64 = 0b001;

/// Closure: pointer to heap-allocated closure object
/// Payload = pointer >> 3
pub const TAG_CLOSURE: u64 = 0b010;

/// String: pointer to heap-allocated string object
/// Payload = pointer >> 3
pub const TAG_STRING: u64 = 0b011;

/// Symbol: pointer to interned symbol in symbol table
/// Payload = symbol table index
pub const TAG_SYMBOL: u64 = 0b100;

/// Vector: pointer to heap-allocated vector
/// Payload = pointer >> 3
pub const TAG_VECTOR: u64 = 0b101;

/// Immediate: special constants and small values
/// Payload encodes the specific immediate type and value
pub const TAG_IMMEDIATE: u64 = 0b110;

/// Box (mutable cell): pointer to heap-allocated box
/// Payload = pointer >> 3
pub const TAG_BOX: u64 = 0b111;

// =============================================================================
// Immediate Sub-tags (stored in payload for TAG_IMMEDIATE values)
// =============================================================================

/// Immediate sub-tag bits (stored in bits 3-7 of payload)
pub const IMMEDIATE_SUBTYPE_BITS: u32 = 5;
pub const IMMEDIATE_SUBTYPE_MASK: u64 = 0b11111;
pub const IMMEDIATE_SUBTYPE_SHIFT: u32 = TAG_BITS;

/// Boolean false: #f
pub const IMMEDIATE_FALSE: u64 = 0b00000;

/// Boolean true: #t
pub const IMMEDIATE_TRUE: u64 = 0b00001;

/// Empty list: '() or nil
pub const IMMEDIATE_NIL: u64 = 0b00010;

/// Void/unspecified value
pub const IMMEDIATE_VOID: u64 = 0b00011;

/// End-of-file object
pub const IMMEDIATE_EOF: u64 = 0b00100;

/// Character: Unicode codepoint in remaining bits
/// Bits 8-31 contain the character value (up to 24 bits, enough for any Unicode)
pub const IMMEDIATE_CHAR: u64 = 0b00101;

// =============================================================================
// Pre-computed Tagged Constants
// =============================================================================

/// The tagged value for #f
pub const VALUE_FALSE: u64 = TAG_IMMEDIATE | (IMMEDIATE_FALSE << IMMEDIATE_SUBTYPE_SHIFT);

/// The tagged value for #t  
pub const VALUE_TRUE: u64 = TAG_IMMEDIATE | (IMMEDIATE_TRUE << IMMEDIATE_SUBTYPE_SHIFT);

/// The tagged value for '() (nil/empty list)
pub const VALUE_NIL: u64 = TAG_IMMEDIATE | (IMMEDIATE_NIL << IMMEDIATE_SUBTYPE_SHIFT);

/// The tagged value for void
pub const VALUE_VOID: u64 = TAG_IMMEDIATE | (IMMEDIATE_VOID << IMMEDIATE_SUBTYPE_SHIFT);

/// The tagged value for EOF
pub const VALUE_EOF: u64 = TAG_IMMEDIATE | (IMMEDIATE_EOF << IMMEDIATE_SUBTYPE_SHIFT);

// =============================================================================
// Helper Functions
// =============================================================================

/// Extract the tag from a tagged value
#[inline]
pub const fn get_tag(value: u64) -> u64 {
    value & TAG_MASK
}

/// Extract the payload from a tagged value (excluding tag bits)
#[inline]
pub const fn get_payload(value: u64) -> u64 {
    value >> PAYLOAD_SHIFT
}

/// Create a tagged fixnum from an i64
/// Note: The value must fit in 61 bits (approximately -2^60 to 2^60)
#[inline]
pub const fn make_fixnum(n: i64) -> u64 {
    ((n as u64) << PAYLOAD_SHIFT) | TAG_FIXNUM
}

/// Extract a fixnum value from a tagged value
#[inline]
pub const fn get_fixnum(value: u64) -> i64 {
    // Arithmetic shift right to sign-extend
    (value as i64) >> PAYLOAD_SHIFT
}

/// Create a tagged pointer value
#[inline]
pub const fn make_pointer(ptr: u64, tag: u64) -> u64 {
    // Pointers should be 8-byte aligned, so low 3 bits are free
    debug_assert!(ptr & TAG_MASK == 0, "Pointer must be 8-byte aligned");
    ptr | tag
}

/// Extract a pointer from a tagged value
#[inline]
pub const fn get_pointer(value: u64) -> u64 {
    value & !TAG_MASK
}

/// Create a tagged character value
#[inline]
pub const fn make_char(c: char) -> u64 {
    let codepoint = c as u64;
    TAG_IMMEDIATE | (IMMEDIATE_CHAR << IMMEDIATE_SUBTYPE_SHIFT) | (codepoint << 8)
}

/// Extract a character from a tagged value
#[inline]
pub const fn get_char(value: u64) -> char {
    let codepoint = (value >> 8) as u32;
    // Safety: We trust that we only create valid char values
    unsafe { char::from_u32_unchecked(codepoint) }
}

/// Check if a value is a fixnum
#[inline]
pub const fn is_fixnum(value: u64) -> bool {
    get_tag(value) == TAG_FIXNUM
}

/// Check if a value is a pair
#[inline]
pub const fn is_pair(value: u64) -> bool {
    get_tag(value) == TAG_PAIR
}

/// Check if a value is a closure
#[inline]
pub const fn is_closure(value: u64) -> bool {
    get_tag(value) == TAG_CLOSURE
}

/// Check if a value is an immediate
#[inline]
pub const fn is_immediate(value: u64) -> bool {
    get_tag(value) == TAG_IMMEDIATE
}

/// Check if a value is nil (empty list)
#[inline]
pub const fn is_nil(value: u64) -> bool {
    value == VALUE_NIL
}

/// Check if a value is boolean false (for conditionals)
/// In Scheme, only #f is false; everything else is true
#[inline]
pub const fn is_false(value: u64) -> bool {
    value == VALUE_FALSE
}

/// Check if a value is truthy (not #f)
#[inline]
pub const fn is_truthy(value: u64) -> bool {
    value != VALUE_FALSE
}

// =============================================================================
// Heap Object Headers
// =============================================================================

/// Reference count field offset in heap objects (in bytes)
pub const REFCOUNT_OFFSET: usize = 0;

/// Reference count field size (in bytes)
pub const REFCOUNT_SIZE: usize = 8;

/// Object type field offset (after refcount)
pub const OBJTYPE_OFFSET: usize = REFCOUNT_SIZE;

/// Object type field size
pub const OBJTYPE_SIZE: usize = 8;

/// Total header size for heap objects
pub const HEADER_SIZE: usize = REFCOUNT_SIZE + OBJTYPE_SIZE;

// =============================================================================
// Pair Layout
// =============================================================================

/// Offset of car field in a pair (after header)
pub const PAIR_CAR_OFFSET: usize = HEADER_SIZE;

/// Offset of cdr field in a pair
pub const PAIR_CDR_OFFSET: usize = HEADER_SIZE + 8;

/// Total size of a pair object
pub const PAIR_SIZE: usize = HEADER_SIZE + 16;

// =============================================================================
// Closure Layout
// =============================================================================

/// Offset of function pointer in closure (after header)
pub const CLOSURE_FUNC_OFFSET: usize = HEADER_SIZE;

/// Offset of captured variable count
pub const CLOSURE_NCAPTURES_OFFSET: usize = HEADER_SIZE + 8;

/// Offset of first captured variable
pub const CLOSURE_CAPTURES_OFFSET: usize = HEADER_SIZE + 16;

// =============================================================================
// Box Layout
// =============================================================================

/// Offset of boxed value (after header)
pub const BOX_VALUE_OFFSET: usize = HEADER_SIZE;

/// Total size of a box object
pub const BOX_SIZE: usize = HEADER_SIZE + 8;

// =============================================================================
// String Layout
// =============================================================================

/// Offset of string length (after header)
pub const STRING_LENGTH_OFFSET: usize = HEADER_SIZE;

/// Offset of string data (UTF-8 bytes)
pub const STRING_DATA_OFFSET: usize = HEADER_SIZE + 8;

// =============================================================================
// Vector Layout
// =============================================================================

/// Offset of vector length (after header)
pub const VECTOR_LENGTH_OFFSET: usize = HEADER_SIZE;

/// Offset of vector data
pub const VECTOR_DATA_OFFSET: usize = HEADER_SIZE + 8;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_fixnum_roundtrip() {
        for n in [-1000, -1, 0, 1, 42, 1000, i32::MAX as i64, i32::MIN as i64] {
            let tagged = make_fixnum(n);
            assert!(is_fixnum(tagged));
            assert_eq!(get_fixnum(tagged), n);
        }
    }

    #[test]
    fn test_constants() {
        assert!(is_false(VALUE_FALSE));
        assert!(!is_false(VALUE_TRUE));
        assert!(is_truthy(VALUE_TRUE));
        assert!(!is_truthy(VALUE_FALSE));
        assert!(is_nil(VALUE_NIL));
        assert!(is_immediate(VALUE_VOID));
    }

    #[test]
    fn test_char_roundtrip() {
        for c in ['a', 'Z', '0', '\n', '❤', '日'] {
            let tagged = make_char(c);
            assert!(is_immediate(tagged));
            assert_eq!(get_char(tagged), c);
        }
    }

    #[test]
    fn test_pointer_roundtrip() {
        // Test 8-byte aligned addresses
        for addr in [0x1000u64, 0x8000, 0x10000, 0xFFFF_FFF8] {
            let tagged = make_pointer(addr, TAG_PAIR);
            assert!(is_pair(tagged));
            assert_eq!(get_pointer(tagged), addr);
        }
    }
}
