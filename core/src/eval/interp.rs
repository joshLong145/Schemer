//! The actual tree-walking machine: `eval_anf`, `eval_complex`, `eval_atom`,
//! `apply_closure`, `apply_tail` (spec §6/§6.1).
//!
//! Tail calls (§6.1): `ComplexExpr::TailApp`/`AnfExpr::TailCall` mark tail
//! position. `step` walks an `AnfExpr` exactly like `eval_anf` except that,
//! on reaching a tail call (directly, or through `If`/`Let`/`Seq` bodies that
//! preserve tail position), it returns `Step::TailInto` instead of
//! recursing. `apply_tail` is a `loop` over `step`, giving O(1) Rust-stack
//! growth for self-recursive tail loops - the loop never recurses into
//! itself; only per-call `Let`/`Seq`/`If` nesting within a single function
//! body recurses, bounded by static source nesting, not iteration count.

use std::rc::Rc;

use crate::compiler::anf::{AnfExpr, Atom, ComplexExpr, VarId};
use crate::eval::primitives::call_prim;
use crate::eval::session::Session;
use crate::eval::value_bridge::atom_to_value;
use crate::interner::Symbol;
use crate::types::{Closure, Env, Value};

/// Result of taking one step through an `AnfExpr`: either a final value, or
/// a tail call to make (to be looped on by the caller, not recursed into).
enum Step {
    Value(Value),
    TailInto { closure: Closure, args: Vec<Value> },
}

/// Evaluate an atom (variable reference or literal) to a `Value`.
pub fn eval_atom(atom: &Atom, env: &Rc<Env>, session: &Session) -> Result<Value, String> {
    match atom {
        Atom::Var(v) => lookup(v, env, session),
        other => atom_to_value(other, session),
    }
}

fn lookup(var: &VarId, env: &Rc<Env>, session: &Session) -> Result<Value, String> {
    env.lookup(var.0)
        .cloned()
        .or_else(|| session.globals.get(&var.0).cloned())
        .ok_or_else(|| format!("undefined variable: {}", session.interner.resolve(var.0)))
}

fn eval_atoms(atoms: &[Atom], env: &Rc<Env>, session: &mut Session) -> Result<Vec<Value>, String> {
    atoms.iter().map(|a| eval_atom(a, env, session)).collect()
}

/// True iff `body` is exactly `Return(Var(var))` - the trivial passthrough
/// shape `transform_application`/`transform_if` always wrap a tail-position
/// `TailApp`/`If` in (`Let{var, value, Return(Var(var))}`). Detected
/// defensively rather than assumed, so a differently-shaped `Let` still
/// evaluates correctly (just without the trampoline fast path).
fn is_trivial_passthrough(body: &AnfExpr, var: &VarId) -> bool {
    matches!(body, AnfExpr::Return(Atom::Var(v)) if v == var)
}

/// Walk `expr`, returning either its value or a tail call to make.
fn step(expr: &AnfExpr, env: &Rc<Env>, session: &mut Session) -> Result<Step, String> {
    match expr {
        AnfExpr::Return(atom) => Ok(Step::Value(eval_atom(atom, env, session)?)),

        AnfExpr::Let { var, value, body } => {
            // Tail call in tail position: trampoline instead of recursing.
            if let ComplexExpr::TailApp { func, args } = value {
                if is_trivial_passthrough(body, var) {
                    let closure = eval_atom(func, env, session)?.expect_closure()?;
                    let arg_values = eval_atoms(args, env, session)?;
                    return Ok(Step::TailInto {
                        closure,
                        args: arg_values,
                    });
                }
            }
            // `If` in tail position: stepping into whichever branch is
            // chosen preserves tail position through conditionals - this is
            // what makes `(if test (loop ...) (loop ...))` trampoline.
            if let ComplexExpr::If {
                cond,
                then_expr,
                else_expr,
            } = value
            {
                if is_trivial_passthrough(body, var) {
                    let c = eval_atom(cond, env, session)?;
                    let branch = if c.is_truthy() { then_expr } else { else_expr };
                    return step(branch, env, session);
                }
            }

            let v = eval_complex(value, env, session)?;
            let env2 = env.extend(vec![(var.0, v)]);
            step(body, &env2, session)
        }

        AnfExpr::Seq { effect, body } => {
            eval_complex(effect, env, session)?;
            step(body, env, session)
        }

        AnfExpr::TailCall { func, args } => {
            let closure = eval_atom(func, env, session)?.expect_closure()?;
            let arg_values = eval_atoms(args, env, session)?;
            Ok(Step::TailInto {
                closure,
                args: arg_values,
            })
        }

        AnfExpr::Halt(atom) => Err(format!("halt: {:?}", eval_atom(atom, env, session)?)),
    }
}

/// Evaluate an `AnfExpr` to a `Value`. Any tail call reached is applied via
/// `apply_tail` (trampolined internally), not native Rust recursion.
pub fn eval_anf(expr: &AnfExpr, env: &Rc<Env>, session: &mut Session) -> Result<Value, String> {
    match step(expr, env, session)? {
        Step::Value(v) => Ok(v),
        Step::TailInto { closure, args } => apply_tail(closure, args, session),
    }
}

/// Evaluate the top-level program entry (`AnfProgram::entry`, i.e.
/// `transform_top_level`'s output). Each top-level `Let`/`Seq` binds into
/// `Session.globals` instead of a local `Env` frame (§5/§11 item 1) - this
/// is what makes a recursive top-level `define` see itself: the closure
/// `(define f (lambda ...))` creates captures `Env::empty()` (nothing is
/// local at top level), so any free-variable reference inside its body -
/// including to `f` itself - falls through to `Session.globals`, which is
/// looked up dynamically at call time, by which point the binding exists,
/// unlike a lexical `Env` frame captured before the binding was added to it.
pub fn eval_toplevel(expr: &AnfExpr, session: &mut Session) -> Result<Value, String> {
    let empty = Env::empty();
    match expr {
        AnfExpr::Let { var, value, body } => {
            let v = eval_complex(value, &empty, session)?;
            session.globals.insert(var.0, v);
            eval_toplevel(body, session)
        }
        AnfExpr::Seq { effect, body } => {
            eval_complex(effect, &empty, session)?;
            eval_toplevel(body, session)
        }
        other => eval_anf(other, &empty, session),
    }
}

/// Evaluate the 9 `ComplexExpr` variants to a `Value` (non-tail; `If`
/// recurses into `eval_anf` on whichever branch, `App` calls `apply_closure`
/// and returns normally - acceptable since tail position is exactly what
/// `TailApp`/`TailCall` exist to protect).
fn eval_complex(expr: &ComplexExpr, env: &Rc<Env>, session: &mut Session) -> Result<Value, String> {
    match expr {
        ComplexExpr::PrimApp { op, args } => {
            let arg_values = eval_atoms(args, env, session)?;
            call_prim(op, arg_values, session)
        }

        ComplexExpr::App { func, args } | ComplexExpr::TailApp { func, args } => {
            let closure = eval_atom(func, env, session)?.expect_closure()?;
            let arg_values = eval_atoms(args, env, session)?;
            apply_closure(&closure, arg_values, session)
        }

        ComplexExpr::MakeClosure { label, captures: _ } => {
            // Captures are ignored: the interpreter closes over the whole
            // lexical `env` chain rather than an explicit free-variable
            // list (spec §4) - `closure.rs`'s conversion is never run here.
            Ok(Value::Closure(Closure {
                label: *label,
                env: env.clone(),
            }))
        }

        // Only ever produced by `closure.rs`, which the interpreter skips
        // entirely (§4) - unreachable via `AnfTransformer`'s raw output.
        ComplexExpr::ClosureRef { .. } => Err(
            "ClosureRef: not supported (closure conversion is not applied for the interpreter)"
                .to_string(),
        ),

        ComplexExpr::If {
            cond,
            then_expr,
            else_expr,
        } => {
            let c = eval_atom(cond, env, session)?;
            let branch = if c.is_truthy() { then_expr } else { else_expr };
            eval_anf(branch, env, session)
        }

        ComplexExpr::MakeBox(atom) => {
            let v = eval_atom(atom, env, session)?;
            Ok(Value::Box(Rc::new(std::cell::RefCell::new(v))))
        }

        ComplexExpr::ReadBox(var) => match lookup(var, env, session)? {
            Value::Box(cell) => Ok(cell.borrow().clone()),
            other => Err(format!("ReadBox: {} is not a box", other)),
        },

        ComplexExpr::WriteBox { box_var, value } => {
            let v = eval_atom(value, env, session)?;
            match lookup(box_var, env, session)? {
                Value::Box(cell) => {
                    *cell.borrow_mut() = v;
                    Ok(Value::Void)
                }
                other => Err(format!("WriteBox: {} is not a box", other)),
            }
        }
    }
}

/// Apply a closure to already-evaluated arguments (ordinary, non-tail call -
/// used by `App` and as the entry point into `apply_tail`'s loop).
pub fn apply_closure(
    closure: &Closure,
    args: Vec<Value>,
    session: &mut Session,
) -> Result<Value, String> {
    apply_tail(closure.clone(), args, session)
}

/// The trampoline (§6.1): loop over function calls instead of recursing, so
/// a chain of tail calls costs O(1) Rust stack regardless of length.
pub fn apply_tail(
    mut closure: Closure,
    mut args: Vec<Value>,
    session: &mut Session,
) -> Result<Value, String> {
    loop {
        let def = session
            .functions
            .get(&closure.label)
            .cloned()
            .ok_or_else(|| {
                format!(
                    "undefined function: {}",
                    session.interner.resolve(closure.label)
                )
            })?;

        if def.params.len() != args.len() {
            return Err(format!(
                "{}: expected {} argument(s), got {}",
                session.interner.resolve(closure.label),
                def.params.len(),
                args.len()
            ));
        }

        let bindings: Vec<(Symbol, Value)> = def
            .params
            .iter()
            .map(|p| p.0)
            .zip(args.into_iter())
            .collect();
        let call_env = closure.env.extend(bindings);

        match step(&def.body, &call_env, session)? {
            Step::Value(v) => return Ok(v),
            Step::TailInto {
                closure: next_closure,
                args: next_args,
            } => {
                closure = next_closure;
                args = next_args;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::compiler::anf::{FunctionDef, PrimOp};
    use crate::interner::Interner;
    use std::collections::HashSet;

    fn empty_env() -> Rc<Env> {
        Env::empty()
    }

    #[test]
    fn return_literal() {
        let mut session = Session::new();
        let expr = AnfExpr::Return(Atom::Int(42));
        let v = eval_anf(&expr, &empty_env(), &mut session).unwrap();
        assert_eq!(v, Value::Number(crate::types::Number::Int(42)));
    }

    #[test]
    fn let_binds_and_returns() {
        let mut session = Session::new();
        let x = VarId::new("x", &mut session.interner);
        let expr = AnfExpr::Let {
            var: x,
            value: ComplexExpr::PrimApp {
                op: PrimOp::Add,
                args: vec![Atom::Int(1), Atom::Int(2)],
            },
            body: Box::new(AnfExpr::Return(Atom::Var(x))),
        };
        let v = eval_anf(&expr, &empty_env(), &mut session).unwrap();
        assert_eq!(v, Value::Number(crate::types::Number::Int(3)));
    }

    #[test]
    fn if_picks_branch() {
        let mut session = Session::new();
        let expr = AnfExpr::Return(Atom::Bool(true));
        // Sanity: exercise eval_complex's If via a Let wrapper.
        let x = VarId::new("x", &mut session.interner);
        let full = AnfExpr::Let {
            var: x,
            value: ComplexExpr::If {
                cond: Atom::Bool(false),
                then_expr: Box::new(AnfExpr::Return(Atom::Int(1))),
                else_expr: Box::new(AnfExpr::Return(Atom::Int(2))),
            },
            body: Box::new(AnfExpr::Return(Atom::Var(x))),
        };
        let v = eval_anf(&full, &empty_env(), &mut session).unwrap();
        assert_eq!(v, Value::Number(crate::types::Number::Int(2)));
        // and the literal-true check above is trivially true
        let _ = eval_anf(&expr, &empty_env(), &mut session).unwrap();
    }

    /// Hand-built self-recursive tail loop:
    /// f(n) = if n == 0 then 0 else f(n - 1)
    /// Constructed directly against `FunctionDef`/`AnfExpr` fixtures (no
    /// parser) to validate the trampoline in isolation.
    #[test]
    fn tail_recursive_loop_is_stack_safe() {
        let mut interner = Interner::new();
        let n = VarId::new("n", &mut interner);
        let label = interner.intern("f");
        let cmp = VarId::temp(0, &mut interner);
        let dec = VarId::temp(1, &mut interner);

        // body: let cmp = (= n 0) in if cmp then 0 else (let dec = (- n 1) in f(dec))
        let body = AnfExpr::Let {
            var: cmp,
            value: ComplexExpr::PrimApp {
                op: PrimOp::NumEq,
                args: vec![Atom::Var(n), Atom::Int(0)],
            },
            body: Box::new(AnfExpr::Let {
                var: VarId::temp(2, &mut interner),
                value: ComplexExpr::If {
                    cond: Atom::Var(cmp),
                    then_expr: Box::new(AnfExpr::Return(Atom::Int(0))),
                    else_expr: Box::new(AnfExpr::Let {
                        var: dec,
                        value: ComplexExpr::PrimApp {
                            op: PrimOp::Sub,
                            args: vec![Atom::Var(n), Atom::Int(1)],
                        },
                        body: Box::new(AnfExpr::Let {
                            var: VarId::temp(3, &mut interner),
                            value: ComplexExpr::TailApp {
                                func: Atom::Var(VarId(label)),
                                args: vec![Atom::Var(dec)],
                            },
                            body: Box::new(AnfExpr::Return(Atom::Var(VarId::temp(
                                3,
                                &mut interner,
                            )))),
                        }),
                    }),
                },
                body: Box::new(AnfExpr::Return(Atom::Var(VarId::temp(2, &mut interner)))),
            }),
        };

        let mut session = Session::new();
        session.interner = interner;
        session.functions.insert(
            label,
            Rc::new(FunctionDef {
                label,
                source_name: Some("f".to_string()),
                params: vec![n],
                has_env: false,
                body,
                free_vars: HashSet::new(),
            }),
        );

        let closure = Closure {
            label,
            env: Env::empty(),
        };
        // The body references `f` as a free variable (self-recursion);
        // resolve it via `Session.globals` (§11 item 1's fallback), since
        // this fixture bypasses `MakeClosure`/closure-capture wiring.
        session.globals.insert(label, Value::Closure(closure.clone()));
        let result = apply_tail(
            closure,
            vec![Value::Number(crate::types::Number::Int(2_000_000))],
            &mut session,
        )
        .unwrap();
        assert_eq!(result, Value::Number(crate::types::Number::Int(0)));
    }
}
