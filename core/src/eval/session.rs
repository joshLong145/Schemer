//! `Session` - the interpreter's persistent state across one or more
//! `AnfProgram`s (today: exactly one, per rollout step 5; multi-program/REPL
//! accumulation is rollout step 7/8, not this pass).

use std::collections::HashMap;
use std::rc::Rc;

use crate::compiler::anf::{AnfTransformer, FunctionDef};
use crate::eval::interp::eval_toplevel;
use crate::interner::{Interner, Symbol};
use crate::proc::ProcedureFn;
use crate::types::Value;

pub struct Session {
    /// Top-level bindings that outlive any single lexical `Env` chain.
    /// Currently unpopulated in single-shot `eval_program` (the whole
    /// program's top-level `define`s are already visible to everything
    /// after them via ordinary lexical `Let` nesting produced by
    /// `transform_top_level`) - this exists for forward compatibility with
    /// REPL mode (§8), where each line's `Env` chain does *not* extend the
    /// previous line's, so cross-line top-level bindings need a home outside
    /// any one line's lexical scope.
    pub globals: HashMap<Symbol, Value>,
    /// Function table, keyed by label. `Rc`, not by-value - `FunctionDef`
    /// owns a full `AnfExpr` body tree, and `apply_tail`'s trampoline looks
    /// this up on every iteration; cloning the `Rc` is a pointer bump, not a
    /// deep copy (§6.1 review finding).
    pub functions: HashMap<Symbol, Rc<FunctionDef>>,
    /// String literal table from the most recently loaded `AnfProgram`.
    pub strings: Vec<String>,
    /// Symbol literal table from the most recently loaded `AnfProgram`.
    pub symbols: Vec<String>,
    /// Extension registration point (§7): `PrimOp::ExtCall` would dispatch
    /// here. Unpopulated/unused until a marshalling shim exists (out of
    /// scope for this pass - no `PrimOp::ExtCall` variant exists yet).
    #[allow(dead_code)]
    pub ext_table: HashMap<String, ProcedureFn>,
    /// Identifier interner, taken from the loaded `AnfProgram` (itself
    /// originating from `AnfTransformer`, see `core/src/interner.rs`).
    pub interner: Interner,
}

impl Session {
    pub fn new() -> Self {
        Self {
            globals: HashMap::new(),
            functions: HashMap::new(),
            strings: Vec::new(),
            symbols: Vec::new(),
            ext_table: HashMap::new(),
            interner: Interner::new(),
        }
    }

    /// Parse output -> `AnfTransformer::transform_program` -> `eval_anf`
    /// (spec §2/§9 step 5), using a fresh `AnfTransformer` per call. Single-
    /// shot only in this pass: each call replaces `strings`/`symbols`/
    /// `interner` wholesale rather than accumulating across calls (that
    /// accumulation, plus reusing one long-lived `AnfTransformer`, is REPL
    /// mode - spec §8/rollout step 7/8, not this pass).
    pub fn eval_program(&mut self, exprs: Vec<Value>) -> Result<Value, String> {
        let mut transformer = AnfTransformer::new();
        let program = transformer.transform_program(exprs)?;

        self.interner = program.interner;
        self.strings = program.strings;
        self.symbols = program.symbols;
        for f in program.functions {
            self.functions.insert(f.label, Rc::new(f));
        }

        eval_toplevel(&program.entry, self)
    }
}

impl Default for Session {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser::read_all;
    use crate::types::Number;

    fn run(src: &str) -> Result<Value, String> {
        let exprs = read_all(src).map_err(|e| e.msg)?;
        Session::new().eval_program(exprs)
    }

    #[test]
    fn arithmetic() {
        assert_eq!(run("(+ 1 2)").unwrap(), Value::Number(Number::Int(3)));
    }

    #[test]
    fn define_and_lambda() {
        assert_eq!(
            run("(define f (lambda (x) (* x x))) (f 4)").unwrap(),
            Value::Number(Number::Int(16))
        );
    }

    #[test]
    fn recursive_factorial() {
        let src = "(define fact (lambda (n) (if (< n 2) 1 (* n (fact (- n 1)))))) (fact 5)";
        assert_eq!(run(src).unwrap(), Value::Number(Number::Int(120)));
    }
}
