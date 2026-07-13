//! Lexical environment representation for the ANF tree-walking interpreter.
//!
//! Lives under `types/` (rather than `eval/`) because `Value::Closure` embeds
//! a `Closure`, which embeds an `Rc<Env>` - the environment is part of the
//! value graph, not just interpreter-internal machinery.

use std::cell::RefCell;
use std::rc::Rc;

use crate::interner::Symbol;
use crate::types::value::Value;

/// A persistent, linked lexical environment. Each frame batches whatever
/// bindings were introduced together (one `Let`, or all of a closure's
/// params at call time) so a call allocates one `Rc` rather than one per
/// parameter.
///
/// `bindings` is a `RefCell` for exactly one reason: tying the recursive
/// knot for `Let`-bound closures (`(define f (lambda ...))` inside a
/// function body, or `letrec`-style self-recursion). The frame is created
/// with a placeholder binding, the closure captures the frame - including
/// its own binding - and the placeholder is then patched to the closure via
/// `rebind`. No other code path mutates a frame after creation.
#[derive(Debug)]
pub enum Env {
    Empty,
    Frame {
        bindings: RefCell<Vec<(Symbol, Value)>>,
        parent: Rc<Env>,
    },
}

impl Env {
    pub fn empty() -> Rc<Env> {
        Rc::new(Env::Empty)
    }

    pub fn lookup(&self, var: Symbol) -> Option<Value> {
        match self {
            Env::Empty => None,
            Env::Frame { bindings, parent } => bindings
                .borrow()
                .iter()
                .rev()
                .find(|(v, _)| *v == var)
                .map(|(_, val)| val.clone())
                .or_else(|| parent.lookup(var)),
        }
    }

    /// One binding (`Let`/`Seq`) or many bound together (a call's params) -
    /// same frame shape either way.
    pub fn extend(self: &Rc<Self>, bindings: Vec<(Symbol, Value)>) -> Rc<Env> {
        Rc::new(Env::Frame {
            bindings: RefCell::new(bindings),
            parent: self.clone(),
        })
    }

    /// Replace an existing binding in *this frame only* (no parent walk).
    /// Used solely to patch a recursive-closure placeholder (see the type
    /// docs). Returns false if the symbol isn't bound in this frame.
    pub fn rebind(&self, var: Symbol, value: Value) -> bool {
        match self {
            Env::Empty => false,
            Env::Frame { bindings, .. } => {
                let mut bindings = bindings.borrow_mut();
                if let Some(slot) = bindings.iter_mut().rev().find(|(v, _)| *v == var) {
                    slot.1 = value;
                    true
                } else {
                    false
                }
            }
        }
    }
}

/// A closure captures a function label (key into `Session.functions`) and
/// the lexical environment in scope at the point the closure was created.
/// Unlike the compiled backend's explicit free-variable capture
/// (`closure.rs`), the interpreter closes over the whole environment chain
/// by reference - cheap (`Rc` clone) and requires zero closure-conversion.
#[derive(Clone, Debug)]
pub struct Closure {
    pub label: Symbol,
    pub env: Rc<Env>,
}

impl PartialEq for Closure {
    fn eq(&self, other: &Self) -> bool {
        // Two closures are the same closure only if they were created at the
        // same site with the same captured environment (identity, like
        // `Procedure`'s `PartialEq`) - not structural/deep equality.
        self.label == other.label && Rc::ptr_eq(&self.env, &other.env)
    }
}
