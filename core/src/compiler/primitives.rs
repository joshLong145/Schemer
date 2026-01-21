//! Primitive operation definitions for the QBE compiler
//!
//! This module defines how primitive Scheme operations map to runtime calls
//! and inline QBE instructions.

use crate::compiler::anf::PrimOp;

/// Classification of how a primitive should be compiled
#[derive(Clone, Debug)]
pub enum PrimImpl {
    /// Inline arithmetic/comparison that can be done with QBE instructions
    Inline(InlineOp),
    /// Call to a runtime function
    RuntimeCall(&'static str),
}

/// Inline operations that map directly to QBE instructions
#[derive(Clone, Debug)]
pub enum InlineOp {
    /// Identity: return the argument unchanged
    Identity,
    /// Addition: add two fixnums
    Add,
    /// Subtraction: subtract two fixnums
    Sub,
    /// Multiplication: multiply two fixnums
    Mul,
    /// Division: divide two fixnums
    Div,
    /// Modulo: remainder of division
    Mod,
    /// Numeric equality: compare two fixnums
    NumEq,
    /// Less than comparison
    Lt,
    /// Greater than comparison
    Gt,
    /// Less than or equal
    Le,
    /// Greater than or equal
    Ge,
    /// Logical not (checks if value is #f)
    Not,
    /// Check if value is nil
    IsNull,
    /// List construction (variadic)
    List,
}

/// Get the implementation for a primitive operation
pub fn get_primitive_impl(op: &PrimOp) -> PrimImpl {
    match op {
        // Identity - just copy the value
        PrimOp::Identity => PrimImpl::Inline(InlineOp::Identity),

        // Inline arithmetic (with overflow checks via runtime)
        PrimOp::Add => PrimImpl::Inline(InlineOp::Add),
        PrimOp::Sub => PrimImpl::Inline(InlineOp::Sub),
        PrimOp::Mul => PrimImpl::Inline(InlineOp::Mul),
        PrimOp::Div => PrimImpl::Inline(InlineOp::Div),
        PrimOp::Mod => PrimImpl::Inline(InlineOp::Mod),

        // Inline comparisons
        PrimOp::NumEq => PrimImpl::Inline(InlineOp::NumEq),
        PrimOp::Lt => PrimImpl::Inline(InlineOp::Lt),
        PrimOp::Gt => PrimImpl::Inline(InlineOp::Gt),
        PrimOp::Le => PrimImpl::Inline(InlineOp::Le),
        PrimOp::Ge => PrimImpl::Inline(InlineOp::Ge),

        // Inline type checks
        PrimOp::Not => PrimImpl::Inline(InlineOp::Not),
        PrimOp::IsNull => PrimImpl::Inline(InlineOp::IsNull),

        // Type predicates - runtime calls for tag checking
        PrimOp::IsPair => PrimImpl::RuntimeCall("scm_is_pair"),
        PrimOp::IsNumber => PrimImpl::RuntimeCall("scm_is_number"),
        PrimOp::IsBool => PrimImpl::RuntimeCall("scm_is_bool"),
        PrimOp::IsSymbol => PrimImpl::RuntimeCall("scm_is_symbol"),
        PrimOp::IsString => PrimImpl::RuntimeCall("scm_is_string"),
        PrimOp::IsProc => PrimImpl::RuntimeCall("scm_is_procedure"),
        PrimOp::IsChar => PrimImpl::RuntimeCall("scm_is_char"),

        // List operations - runtime calls (require allocation)
        PrimOp::Cons => PrimImpl::RuntimeCall("scm_cons"),
        PrimOp::Car => PrimImpl::RuntimeCall("scm_car"),
        PrimOp::Cdr => PrimImpl::RuntimeCall("scm_cdr"),
        PrimOp::SetCar => PrimImpl::RuntimeCall("scm_set_car"),
        PrimOp::SetCdr => PrimImpl::RuntimeCall("scm_set_cdr"),

        // Equality - runtime calls
        PrimOp::Eq => PrimImpl::RuntimeCall("scm_eq"),
        PrimOp::Eqv => PrimImpl::RuntimeCall("scm_eqv"),

        // I/O - runtime calls
        PrimOp::Display => PrimImpl::RuntimeCall("scm_display"),
        PrimOp::Newline => PrimImpl::RuntimeCall("scm_newline"),

        // List construction - special handling in codegen (variadic)
        PrimOp::List => PrimImpl::Inline(InlineOp::List),
    }
}

/// Information about a runtime function
#[derive(Clone, Debug)]
pub struct RuntimeFn {
    /// Name of the runtime function
    pub name: &'static str,
    /// Number of parameters
    pub arity: usize,
    /// Whether this function can trigger GC
    pub can_gc: bool,
    /// Whether this function can raise an exception
    pub can_raise: bool,
}

/// Runtime function definitions
pub static RUNTIME_FUNCTIONS: &[RuntimeFn] = &[
    // Type predicates
    RuntimeFn {
        name: "scm_is_pair",
        arity: 1,
        can_gc: false,
        can_raise: false,
    },
    RuntimeFn {
        name: "scm_is_number",
        arity: 1,
        can_gc: false,
        can_raise: false,
    },
    RuntimeFn {
        name: "scm_is_bool",
        arity: 1,
        can_gc: false,
        can_raise: false,
    },
    RuntimeFn {
        name: "scm_is_symbol",
        arity: 1,
        can_gc: false,
        can_raise: false,
    },
    RuntimeFn {
        name: "scm_is_string",
        arity: 1,
        can_gc: false,
        can_raise: false,
    },
    RuntimeFn {
        name: "scm_is_procedure",
        arity: 1,
        can_gc: false,
        can_raise: false,
    },
    RuntimeFn {
        name: "scm_is_char",
        arity: 1,
        can_gc: false,
        can_raise: false,
    },
    // List operations
    RuntimeFn {
        name: "scm_cons",
        arity: 2,
        can_gc: true,
        can_raise: false,
    },
    RuntimeFn {
        name: "scm_car",
        arity: 1,
        can_gc: false,
        can_raise: true,
    },
    RuntimeFn {
        name: "scm_cdr",
        arity: 1,
        can_gc: false,
        can_raise: true,
    },
    RuntimeFn {
        name: "scm_set_car",
        arity: 2,
        can_gc: false,
        can_raise: true,
    },
    RuntimeFn {
        name: "scm_set_cdr",
        arity: 2,
        can_gc: false,
        can_raise: true,
    },
    // Equality
    RuntimeFn {
        name: "scm_eq",
        arity: 2,
        can_gc: false,
        can_raise: false,
    },
    RuntimeFn {
        name: "scm_eqv",
        arity: 2,
        can_gc: false,
        can_raise: false,
    },
    // I/O
    RuntimeFn {
        name: "scm_display",
        arity: 1,
        can_gc: false,
        can_raise: true,
    },
    RuntimeFn {
        name: "scm_newline",
        arity: 0,
        can_gc: false,
        can_raise: true,
    },
    // Memory management
    RuntimeFn {
        name: "scm_incref",
        arity: 1,
        can_gc: false,
        can_raise: false,
    },
    RuntimeFn {
        name: "scm_decref",
        arity: 1,
        can_gc: true,
        can_raise: false,
    },
    // Allocation
    RuntimeFn {
        name: "scm_alloc_pair",
        arity: 0,
        can_gc: true,
        can_raise: false,
    },
    RuntimeFn {
        name: "scm_alloc_closure",
        arity: 2,
        can_gc: true,
        can_raise: false,
    },
    RuntimeFn {
        name: "scm_alloc_string",
        arity: 2,
        can_gc: true,
        can_raise: false,
    },
    RuntimeFn {
        name: "scm_alloc_box",
        arity: 0,
        can_gc: true,
        can_raise: false,
    },
    // Box operations
    RuntimeFn {
        name: "scm_box_ref",
        arity: 1,
        can_gc: false,
        can_raise: true,
    },
    RuntimeFn {
        name: "scm_box_set",
        arity: 2,
        can_gc: false,
        can_raise: true,
    },
    // Closure operations
    RuntimeFn {
        name: "scm_closure_ref",
        arity: 2,
        can_gc: false,
        can_raise: false,
    },
    RuntimeFn {
        name: "scm_closure_set",
        arity: 3,
        can_gc: false,
        can_raise: false,
    },
    RuntimeFn {
        name: "scm_closure_func",
        arity: 1,
        can_gc: false,
        can_raise: false,
    },
    // Exception handling
    RuntimeFn {
        name: "scm_raise",
        arity: 1,
        can_gc: false,
        can_raise: true,
    },
    RuntimeFn {
        name: "scm_push_handler",
        arity: 1,
        can_gc: false,
        can_raise: false,
    },
    RuntimeFn {
        name: "scm_pop_handler",
        arity: 0,
        can_gc: false,
        can_raise: false,
    },
    // Trampoline support
    RuntimeFn {
        name: "scm_make_thunk",
        arity: 2,
        can_gc: true,
        can_raise: false,
    },
    RuntimeFn {
        name: "scm_is_thunk",
        arity: 1,
        can_gc: false,
        can_raise: false,
    },
    RuntimeFn {
        name: "scm_thunk_func",
        arity: 1,
        can_gc: false,
        can_raise: false,
    },
    RuntimeFn {
        name: "scm_thunk_args",
        arity: 1,
        can_gc: false,
        can_raise: false,
    },
    // String operations
    RuntimeFn {
        name: "scm_string_length",
        arity: 1,
        can_gc: false,
        can_raise: true,
    },
    RuntimeFn {
        name: "scm_string_ref",
        arity: 2,
        can_gc: false,
        can_raise: true,
    },
    RuntimeFn {
        name: "scm_string_append",
        arity: 2,
        can_gc: true,
        can_raise: true,
    },
    // Symbol operations
    RuntimeFn {
        name: "scm_intern_symbol",
        arity: 1,
        can_gc: true,
        can_raise: false,
    },
    RuntimeFn {
        name: "scm_symbol_to_string",
        arity: 1,
        can_gc: true,
        can_raise: true,
    },
    // Initialization
    RuntimeFn {
        name: "scm_init",
        arity: 0,
        can_gc: false,
        can_raise: false,
    },
    RuntimeFn {
        name: "scm_shutdown",
        arity: 0,
        can_gc: true,
        can_raise: false,
    },
];

/// Get runtime function info by name
pub fn get_runtime_fn(name: &str) -> Option<&'static RuntimeFn> {
    RUNTIME_FUNCTIONS.iter().find(|f| f.name == name)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_primitives_have_impl() {
        // Ensure all PrimOps have an implementation
        let ops = [
            PrimOp::Add,
            PrimOp::Sub,
            PrimOp::Mul,
            PrimOp::Div,
            PrimOp::Mod,
            PrimOp::NumEq,
            PrimOp::Lt,
            PrimOp::Gt,
            PrimOp::Le,
            PrimOp::Ge,
            PrimOp::IsNull,
            PrimOp::IsPair,
            PrimOp::IsNumber,
            PrimOp::IsBool,
            PrimOp::IsSymbol,
            PrimOp::IsString,
            PrimOp::IsProc,
            PrimOp::IsChar,
            PrimOp::Cons,
            PrimOp::Car,
            PrimOp::Cdr,
            PrimOp::SetCar,
            PrimOp::SetCdr,
            PrimOp::Eq,
            PrimOp::Eqv,
            PrimOp::Display,
            PrimOp::Newline,
            PrimOp::Not,
        ];

        for op in &ops {
            let _impl = get_primitive_impl(op);
            // Just making sure it doesn't panic
        }
    }

    #[test]
    fn test_runtime_fn_lookup() {
        assert!(get_runtime_fn("scm_cons").is_some());
        assert!(get_runtime_fn("scm_car").is_some());
        assert!(get_runtime_fn("nonexistent").is_none());

        let cons = get_runtime_fn("scm_cons").unwrap();
        assert_eq!(cons.arity, 2);
        assert!(cons.can_gc);
    }
}
