//! Tag definitions for the runtime
//!
//! This is a copy of the shared tag definitions, kept in sync with
//! src/tags.rs in the main crate. In the future, these could be generated
//! from a single source.

// Re-export everything from the shared tags module pattern
// For now, we duplicate the constants since this is a separate crate

/// Tag bits occupy the low 3 bits of a tagged value
pub const TAG_BITS: u32 = 3;

/// Mask to extract the tag from a tagged value
pub const TAG_MASK: u64 = 0b111;

/// Mask to extract the payload (shift right by TAG_BITS)
pub const PAYLOAD_SHIFT: u32 = TAG_BITS;

// Primary Tags
pub const TAG_FIXNUM: u64 = 0b000;
pub const TAG_PAIR: u64 = 0b001;
pub const TAG_CLOSURE: u64 = 0b010;
pub const TAG_STRING: u64 = 0b011;
pub const TAG_SYMBOL: u64 = 0b100;
pub const TAG_VECTOR: u64 = 0b101;
pub const TAG_IMMEDIATE: u64 = 0b110;
pub const TAG_BOX: u64 = 0b111;

// Immediate Sub-tags
pub const IMMEDIATE_SUBTYPE_SHIFT: u32 = TAG_BITS;
pub const IMMEDIATE_FALSE: u64 = 0b00000;
pub const IMMEDIATE_TRUE: u64 = 0b00001;
pub const IMMEDIATE_NIL: u64 = 0b00010;
pub const IMMEDIATE_VOID: u64 = 0b00011;
pub const IMMEDIATE_EOF: u64 = 0b00100;
pub const IMMEDIATE_CHAR: u64 = 0b00101;

// Pre-computed constants
pub const VALUE_FALSE: u64 = TAG_IMMEDIATE | (IMMEDIATE_FALSE << IMMEDIATE_SUBTYPE_SHIFT);
pub const VALUE_TRUE: u64 = TAG_IMMEDIATE | (IMMEDIATE_TRUE << IMMEDIATE_SUBTYPE_SHIFT);
pub const VALUE_NIL: u64 = TAG_IMMEDIATE | (IMMEDIATE_NIL << IMMEDIATE_SUBTYPE_SHIFT);
pub const VALUE_VOID: u64 = TAG_IMMEDIATE | (IMMEDIATE_VOID << IMMEDIATE_SUBTYPE_SHIFT);
pub const VALUE_EOF: u64 = TAG_IMMEDIATE | (IMMEDIATE_EOF << IMMEDIATE_SUBTYPE_SHIFT);

// Helper functions
#[inline]
pub const fn get_tag(value: u64) -> u64 {
    value & TAG_MASK
}

#[inline]
pub const fn get_payload(value: u64) -> u64 {
    value >> PAYLOAD_SHIFT
}

#[inline]
pub const fn make_fixnum(n: i64) -> u64 {
    ((n as u64) << PAYLOAD_SHIFT) | TAG_FIXNUM
}

#[inline]
pub const fn get_fixnum(value: u64) -> i64 {
    (value as i64) >> PAYLOAD_SHIFT
}

#[inline]
pub const fn make_pointer(ptr: u64, tag: u64) -> u64 {
    ptr | tag
}

#[inline]
pub const fn get_pointer(value: u64) -> u64 {
    value & !TAG_MASK
}

#[inline]
pub const fn is_fixnum(value: u64) -> bool {
    get_tag(value) == TAG_FIXNUM
}

#[inline]
pub const fn is_pair(value: u64) -> bool {
    get_tag(value) == TAG_PAIR
}

#[inline]
pub const fn is_closure(value: u64) -> bool {
    get_tag(value) == TAG_CLOSURE
}

#[inline]
pub const fn is_immediate(value: u64) -> bool {
    get_tag(value) == TAG_IMMEDIATE
}

#[inline]
pub const fn is_nil(value: u64) -> bool {
    value == VALUE_NIL
}

#[inline]
pub const fn is_false(value: u64) -> bool {
    value == VALUE_FALSE
}

#[inline]
pub const fn is_truthy(value: u64) -> bool {
    value != VALUE_FALSE
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

// Heap object layout constants
pub const REFCOUNT_OFFSET: usize = 0;
pub const REFCOUNT_SIZE: usize = 8;
pub const OBJTYPE_OFFSET: usize = REFCOUNT_SIZE;
pub const OBJTYPE_SIZE: usize = 8;
pub const HEADER_SIZE: usize = REFCOUNT_SIZE + OBJTYPE_SIZE;

pub const PAIR_CAR_OFFSET: usize = HEADER_SIZE;
pub const PAIR_CDR_OFFSET: usize = HEADER_SIZE + 8;
pub const PAIR_SIZE: usize = HEADER_SIZE + 16;

pub const CLOSURE_FUNC_OFFSET: usize = HEADER_SIZE;
pub const CLOSURE_NCAPTURES_OFFSET: usize = HEADER_SIZE + 8;
pub const CLOSURE_CAPTURES_OFFSET: usize = HEADER_SIZE + 16;

pub const BOX_VALUE_OFFSET: usize = HEADER_SIZE;
pub const BOX_SIZE: usize = HEADER_SIZE + 8;

pub const STRING_LENGTH_OFFSET: usize = HEADER_SIZE;
pub const STRING_DATA_OFFSET: usize = HEADER_SIZE + 8;

pub const VECTOR_LENGTH_OFFSET: usize = HEADER_SIZE;
pub const VECTOR_DATA_OFFSET: usize = HEADER_SIZE + 8;
