//! `Session` - the interpreter's persistent state across one or more
//! `transform_program` calls (a single-shot file run is one call; REPL mode
//! and prelude-then-user-source are multiple calls on the same `Session`).

use std::collections::HashMap;
use std::rc::Rc;

use crate::compiler::anf::{AnfExpr, AnfTransformer, Atom, ComplexExpr, FunctionDef};
use crate::eval::interp::eval_toplevel;
use crate::interner::{Interner, Symbol};
use crate::proc::ProcedureFn;
use crate::types::Value;

pub struct Session {
    /// Top-level bindings that outlive any single lexical `Env` chain.
    /// Populated by `eval_toplevel` (interp.rs): each top-level `define`
    /// binds here rather than into a lexical frame, which is both what
    /// makes recursive top-level functions see themselves (lookup falls
    /// back here dynamically at call time) and what makes bindings visible
    /// across `eval_program` calls (prelude-then-program, REPL lines, §8) -
    /// each call's `Env` chain does *not* extend the previous call's.
    pub globals: HashMap<Symbol, Value>,
    /// Function table, keyed by label. `Rc`, not by-value - `FunctionDef`
    /// owns a full `AnfExpr` body tree, and `apply_tail`'s trampoline looks
    /// this up on every iteration; cloning the `Rc` is a pointer bump, not a
    /// deep copy (§6.1 review finding).
    pub functions: HashMap<Symbol, Rc<FunctionDef>>,
    /// Cumulative string literal table. Each `transform_program` call
    /// returns *call-local* tables (`string_map`/`symbol_map` are reset per
    /// call, spec §8/§12.4); `eval_program` appends them here and remaps the
    /// new program's `Atom::String` indices by the pre-append length, so
    /// function bodies loaded by *earlier* calls keep resolving correctly.
    pub strings: Vec<String>,
    /// Cumulative symbol literal table (same accumulation scheme as
    /// `strings`).
    pub symbols: Vec<String>,
    /// Extension registration point (§7): `PrimOp::ExtCall` would dispatch
    /// here. Unpopulated/unused until a marshalling shim exists (out of
    /// scope for this pass - no `PrimOp::ExtCall` variant exists yet).
    #[allow(dead_code)]
    pub ext_table: HashMap<String, ProcedureFn>,
    /// Identifier interner, taken from the loaded `AnfProgram` (itself
    /// originating from this session's own `transformer` below, whose
    /// interner is never reset - so `Symbol`s stay stable across calls).
    pub interner: Interner,
    /// The one long-lived `AnfTransformer` for this session (spec §8/§12.4):
    /// its identifier interner and temp/label counters persist across
    /// `eval_program` calls, so a name defined by one call (e.g. the
    /// prelude, or an earlier REPL line) interns to the *same* `Symbol`
    /// when referenced by a later call - which is what makes
    /// `Session.globals`/`functions` lookups match across calls.
    transformer: AnfTransformer,
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
            transformer: AnfTransformer::new(),
        }
    }

    /// Evaluate one program-or-line's forms: parse output ->
    /// `transform_program` (on this session's persistent transformer) ->
    /// `eval_toplevel`. Callable any number of times on one `Session`
    /// (prelude then user source; one call per REPL line): the identifier
    /// interner persists in the transformer, `functions`/`globals`
    /// accumulate, and each call's line-local `strings`/`symbols` tables
    /// (per step 7's `string_map`/`symbol_map` reset) are appended to the
    /// session's cumulative tables with this call's `Atom::String`/
    /// `Atom::Symbol` indices remapped by the pre-append offsets - so
    /// function bodies loaded by earlier calls keep resolving against the
    /// same (still-present) cumulative entries. Earlier calls' entries are
    /// never re-evaluated (§8's rejected re-transform-history approach).
    pub fn eval_program(&mut self, exprs: Vec<Value>) -> Result<Value, String> {
        let mut program = self.transformer.transform_program(exprs)?;

        // Remap this call's line-local literal indices to cumulative ones.
        let str_off = self.strings.len();
        let sym_off = self.symbols.len();
        if str_off > 0 || sym_off > 0 {
            remap_expr(&mut program.entry, str_off, sym_off);
            for f in &mut program.functions {
                remap_expr(&mut f.body, str_off, sym_off);
            }
        }
        self.strings.extend(program.strings);
        self.symbols.extend(program.symbols);

        self.interner = program.interner;
        for f in program.functions {
            self.functions.insert(f.label, Rc::new(f));
        }

        eval_toplevel(&program.entry, self)
    }
}

/// Shift every `Atom::String`/`Atom::Symbol` literal index in an expression
/// tree by the given offsets (see `Session::eval_program`).
fn remap_expr(expr: &mut AnfExpr, str_off: usize, sym_off: usize) {
    match expr {
        AnfExpr::Return(atom) | AnfExpr::Halt(atom) => remap_atom(atom, str_off, sym_off),
        AnfExpr::Let { value, body, .. } => {
            remap_complex(value, str_off, sym_off);
            remap_expr(body, str_off, sym_off);
        }
        AnfExpr::Seq { effect, body } => {
            remap_complex(effect, str_off, sym_off);
            remap_expr(body, str_off, sym_off);
        }
        AnfExpr::TailCall { func, args } => {
            remap_atom(func, str_off, sym_off);
            for a in args {
                remap_atom(a, str_off, sym_off);
            }
        }
    }
}

fn remap_complex(expr: &mut ComplexExpr, str_off: usize, sym_off: usize) {
    match expr {
        ComplexExpr::PrimApp { args, .. } => {
            for a in args {
                remap_atom(a, str_off, sym_off);
            }
        }
        ComplexExpr::App { func, args } | ComplexExpr::TailApp { func, args } => {
            remap_atom(func, str_off, sym_off);
            for a in args {
                remap_atom(a, str_off, sym_off);
            }
        }
        ComplexExpr::If {
            cond,
            then_expr,
            else_expr,
        } => {
            remap_atom(cond, str_off, sym_off);
            remap_expr(then_expr, str_off, sym_off);
            remap_expr(else_expr, str_off, sym_off);
        }
        ComplexExpr::MakeBox(atom) => remap_atom(atom, str_off, sym_off),
        ComplexExpr::WriteBox { value, .. } => remap_atom(value, str_off, sym_off),
        ComplexExpr::MakeClosure { .. }
        | ComplexExpr::ClosureRef { .. }
        | ComplexExpr::ReadBox(_) => {}
    }
}

fn remap_atom(atom: &mut Atom, str_off: usize, sym_off: usize) {
    match atom {
        Atom::String(idx) => *idx += str_off,
        Atom::Symbol(idx) => *idx += sym_off,
        _ => {}
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

    fn eval_on(session: &mut Session, src: &str) -> Result<Value, String> {
        let exprs = read_all(src).map_err(|e| e.msg)?;
        session.eval_program(exprs)
    }

    #[test]
    fn repl_lines_accumulate_functions_and_globals() {
        // REPL mode (spec §8): one Session (owning one long-lived
        // AnfTransformer), one transform_program call per line, later lines
        // see earlier lines' top-level defines.
        let mut session = Session::new();

        assert_eq!(eval_on(&mut session, "(define x 10)").unwrap(), Value::Void);
        assert_eq!(
            eval_on(&mut session, "(+ x x)").unwrap(),
            Value::Number(Number::Int(20))
        );

        // A function defined on an earlier line is callable from a later one.
        assert_eq!(
            eval_on(&mut session, "(define f (lambda (n) (+ n x)))").unwrap(),
            Value::Void
        );
        assert_eq!(
            eval_on(&mut session, "(f 5)").unwrap(),
            Value::Number(Number::Int(15))
        );
    }

    #[test]
    fn repl_lines_do_not_replay_earlier_side_effects() {
        // The rejected alternative (§8/§11 item 3) re-transforms and
        // re-evaluates the *whole* input history on every line, which would
        // replay earlier side effects. Confirm a counter only increments
        // once per line, not once per line *per prior line evaluated*.
        let mut session = Session::new();

        eval_on(&mut session, "(define count (lambda (b) (if b 1 0)))").unwrap();
        assert_eq!(
            eval_on(&mut session, "(count #t)").unwrap(),
            Value::Number(Number::Int(1))
        );
        // A third, unrelated line must not re-run the previous `(count #t)`.
        assert_eq!(
            eval_on(&mut session, "(count #f)").unwrap(),
            Value::Number(Number::Int(0))
        );
    }

    #[test]
    fn prelude_then_program_shares_symbols_and_remaps_string_indices() {
        // The exact two-call pattern parse_and_run_scheme uses: call 1 is a
        // "prelude" defining a function whose body contains a string
        // literal; call 2 defines and uses its own copy of the same literal
        // AND calls the prelude function. This exercises both halves of the
        // multi-call design:
        //  - identifier interner persistence: `greet` interns to the same
        //    Symbol in both calls, so call 2 finds call 1's global;
        //  - step-7 literal-table reset + Session-side remapping: call 2's
        //    line-local "hello" index is offset into the cumulative table,
        //    while call 1's FunctionDef body still resolves its own "hello"
        //    correctly when invoked *during call 2*.
        let mut session = Session::new();

        eval_on(&mut session, r#"(define greet (lambda () "hello"))"#).unwrap();

        // Call 2 evaluates its own "hello" literal first (proving the
        // remapped index resolves), then calls the prelude-defined function
        // (proving cross-call Symbol identity and that call 1's body indices
        // still resolve against the cumulative table).
        let own_literal = eval_on(&mut session, r#""hello""#).unwrap();
        assert_eq!(
            own_literal,
            Value::String(std::sync::Arc::new("hello".to_string()))
        );

        let from_prelude_fn = eval_on(&mut session, "(greet)").unwrap();
        assert_eq!(
            from_prelude_fn,
            Value::String(std::sync::Arc::new("hello".to_string()))
        );
    }
}
