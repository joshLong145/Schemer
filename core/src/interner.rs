//! Shared string interning for identifiers used across the compiler pipeline
//! (`anf.rs`, `closure.rs`, `codegen.rs`) and, in the future, the tree-walking
//! interpreter.
//!
//! `Interner` is deliberately *not* a global/static singleton (unlike the
//! runtime's `static mut SYMBOL_TABLE`, which has no natural single owner
//! because compiled Scheme programs need process-wide symbol identity at
//! arbitrary points). Here there is always a natural owner: the
//! `AnfTransformer` that creates it, which threads it through the rest of the
//! pipeline by value inside `AnfProgram`.

use std::collections::HashMap;

/// An interned identifier: a small, `Copy`, cheaply-comparable handle into an
/// `Interner`'s string table.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub struct Symbol(u32);

/// Deduplicating string table. Repeated names (e.g. `"x"` appearing in many
/// lambdas) share one `Symbol`.
#[derive(Default, Clone, Debug)]
pub struct Interner {
    strings: Vec<String>,
    map: HashMap<String, Symbol>,
}

impl Interner {
    pub fn new() -> Self {
        Self::default()
    }

    /// Intern `s`, returning its `Symbol`. Repeated calls with the same
    /// string return the same `Symbol`.
    pub fn intern(&mut self, s: &str) -> Symbol {
        if let Some(&sym) = self.map.get(s) {
            return sym;
        }
        let sym = Symbol(self.strings.len() as u32);
        self.strings.push(s.to_string());
        self.map.insert(s.to_string(), sym);
        sym
    }

    /// Resolve a `Symbol` back to its string. Panics if `sym` was not
    /// produced by this `Interner`.
    pub fn resolve(&self, sym: Symbol) -> &str {
        &self.strings[sym.0 as usize]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn intern_dedups() {
        let mut i = Interner::new();
        let a = i.intern("x");
        let b = i.intern("x");
        assert_eq!(a, b);
    }

    #[test]
    fn intern_distinguishes_different_strings() {
        let mut i = Interner::new();
        let a = i.intern("x");
        let b = i.intern("y");
        assert_ne!(a, b);
    }

    #[test]
    fn resolve_roundtrips() {
        let mut i = Interner::new();
        let sym = i.intern("hello");
        assert_eq!(i.resolve(sym), "hello");
    }
}
