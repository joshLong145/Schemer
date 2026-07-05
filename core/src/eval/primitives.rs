//! Native (non-ANF) implementations backing `PrimOp` (spec §7).
//!
//! `and`/`or` are deliberately absent: they are not `PrimOp`s at all - the
//! ANF transformer (`transform_and`/`transform_or`) desugars them to `If`
//! before the interpreter ever sees them, so there is exactly one
//! implementation of their short-circuiting/variadic semantics, shared by
//! both backends.

use std::sync::Arc;

use crate::compiler::anf::PrimOp;
use crate::compiler::primitives::{get_primitive_impl, get_runtime_fn, PrimImpl};
use crate::eval::session::Session;
use crate::types::{Number, SchemeList, Value};

/// How many arguments a `PrimOp` accepts.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Arity {
    Exact(usize),
    AtLeast(usize),
}

impl Arity {
    fn matches(&self, n: usize) -> bool {
        match self {
            Arity::Exact(k) => n == *k,
            Arity::AtLeast(k) => n >= *k,
        }
    }
}

impl std::fmt::Display for Arity {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Arity::Exact(k) => write!(f, "exactly {}", k),
            Arity::AtLeast(k) => write!(f, "at least {}", k),
        }
    }
}

/// Single source of truth for `PrimOp` arity, reusing the compiler's
/// `get_primitive_impl`/`get_runtime_fn` for `RuntimeCall`-backed ops instead
/// of duplicating an arity check per arm (the duplication that let `and`/`or`
/// drift between `env.rs` and `anf.rs` in the first place).
fn primop_arity(op: &PrimOp) -> Arity {
    match get_primitive_impl(op) {
        PrimImpl::RuntimeCall(name) => Arity::Exact(
            get_runtime_fn(name)
                .unwrap_or_else(|| panic!("no RuntimeFn entry for '{}'", name))
                .arity,
        ),
        PrimImpl::Inline(_) => match op {
            // Identity is a single-arg internal op used by ANF normalization
            // (e.g. wrapping an atom as `PrimApp{Identity, [atom]}`) - not
            // listed in the spec's illustrative arity table, but it is
            // unambiguously unary; folding it into the binary catch-all
            // below would be wrong.
            PrimOp::Identity => Arity::Exact(1),
            PrimOp::Not | PrimOp::IsNull => Arity::Exact(1),
            PrimOp::List => Arity::AtLeast(0),
            _ => Arity::Exact(2), // Add/Sub/Mul/Div/Mod/NumEq/Lt/Gt/Le/Ge - all binary
        },
    }
}

fn check_arity(op: &PrimOp, n: usize) -> Result<(), String> {
    let arity = primop_arity(op);
    if arity.matches(n) {
        Ok(())
    } else {
        Err(format!(
            "{:?}: expected {} argument(s), got {}",
            op, arity, n
        ))
    }
}

fn expect_number(v: &Value, who: &str) -> Result<Number, String> {
    match v {
        Value::Number(n) => Ok(n.clone()),
        other => Err(format!("{}: expected a number, got {}", who, other)),
    }
}

fn numeric_binop(
    args: &[Value],
    who: &str,
    f: impl FnOnce(&Number, &Number) -> Result<Number, String>,
) -> Result<Value, String> {
    let a = expect_number(&args[0], who)?;
    let b = expect_number(&args[1], who)?;
    Ok(Value::Number(f(&a, &b)?))
}

fn numeric_cmp(
    args: &[Value],
    who: &str,
    f: impl FnOnce(&Number, &Number) -> bool,
) -> Result<Value, String> {
    let a = expect_number(&args[0], who)?;
    let b = expect_number(&args[1], who)?;
    Ok(Value::Boolean(f(&a, &b)))
}

fn modulo(a: &Number, b: &Number) -> Result<Number, String> {
    match (a, b) {
        (_, Number::Int(0)) => Err("modulo: division by zero".to_string()),
        (_, Number::Float(b)) if *b == 0.0 => Err("modulo: division by zero".to_string()),
        (Number::Int(a), Number::Int(b)) => Ok(Number::Int(a.rem_euclid(*b))),
        (a, b) => {
            let a = match a {
                Number::Int(i) => *i as f64,
                Number::Float(f) => *f,
            };
            let b = match b {
                Number::Int(i) => *i as f64,
                Number::Float(f) => *f,
            };
            Ok(Number::Float(a.rem_euclid(b)))
        }
    }
}

/// Deep equality (used for both `Eq` and `Eqv` in this pass - our small
/// interpreter doesn't distinguish `eq?`/`eqv?` exactness edge cases the way
/// full R7RS does; see final report for this simplification).
fn values_eqv(a: &Value, b: &Value) -> bool {
    match (a, b) {
        (Value::Number(n1), Value::Number(n2)) => n1 == n2,
        (Value::Boolean(b1), Value::Boolean(b2)) => b1 == b2,
        (Value::Char(c1), Value::Char(c2)) => c1 == c2,
        (Value::Symbol(s1), Value::Symbol(s2)) => s1 == s2,
        (Value::Nil, Value::Nil) => true,
        (Value::Void, Value::Void) => true,
        (Value::String(s1), Value::String(s2)) => Arc::ptr_eq(s1, s2),
        (Value::Pair(p1), Value::Pair(p2)) => Arc::ptr_eq(p1, p2),
        (Value::List(l1), Value::List(l2)) => Arc::ptr_eq(l1, l2),
        (Value::Nil, Value::List(l)) | (Value::List(l), Value::Nil) => l.is_empty(),
        (Value::Box(b1), Value::Box(b2)) => std::rc::Rc::ptr_eq(b1, b2),
        (Value::Closure(c1), Value::Closure(c2)) => c1 == c2,
        _ => false,
    }
}

fn vec_to_list(vals: Vec<Value>) -> Value {
    if vals.is_empty() {
        Value::Nil
    } else {
        Value::List(Arc::new(SchemeList::from_vec(vals)))
    }
}

/// Dispatch a primitive application. `args` are already-evaluated `Value`s.
pub fn call_prim(op: &PrimOp, args: Vec<Value>, _session: &mut Session) -> Result<Value, String> {
    check_arity(op, args.len())?;

    match op {
        PrimOp::Identity => Ok(args.into_iter().next().unwrap()),

        PrimOp::Add => numeric_binop(&args, "+", |a, b| Ok(a.add(b))),
        PrimOp::Sub => numeric_binop(&args, "-", |a, b| Ok(a.sub(b))),
        PrimOp::Mul => numeric_binop(&args, "*", |a, b| Ok(a.mul(b))),
        PrimOp::Div => numeric_binop(&args, "/", |a, b| a.div(b)),
        PrimOp::Mod => numeric_binop(&args, "modulo", modulo),

        PrimOp::NumEq => numeric_cmp(&args, "=", |a, b| a == b),
        PrimOp::Lt => numeric_cmp(&args, "<", |a, b| a < b),
        PrimOp::Gt => numeric_cmp(&args, ">", |a, b| a > b),
        PrimOp::Le => numeric_cmp(&args, "<=", |a, b| a <= b),
        PrimOp::Ge => numeric_cmp(&args, ">=", |a, b| a >= b),

        PrimOp::IsNull => Ok(Value::Boolean(match &args[0] {
            Value::Nil => true,
            Value::List(list) => list.is_empty(),
            _ => false,
        })),
        PrimOp::IsPair => Ok(Value::Boolean(matches!(
            args[0],
            Value::Pair(_) | Value::List(_)
        ))),
        PrimOp::IsNumber => Ok(Value::Boolean(matches!(args[0], Value::Number(_)))),
        PrimOp::IsBool => Ok(Value::Boolean(matches!(args[0], Value::Boolean(_)))),
        PrimOp::IsSymbol => Ok(Value::Boolean(matches!(args[0], Value::Symbol(_)))),
        PrimOp::IsString => Ok(Value::Boolean(matches!(args[0], Value::String(_)))),
        PrimOp::IsProc => Ok(Value::Boolean(matches!(args[0], Value::Closure(_)))),
        PrimOp::IsChar => Ok(Value::Boolean(matches!(args[0], Value::Char(_)))),

        PrimOp::Cons => Ok(Value::Pair(Arc::new((args[0].clone(), args[1].clone())))),
        PrimOp::Car => match &args[0] {
            Value::Pair(p) => Ok(p.0.clone()),
            Value::List(list) => list
                .car()
                .cloned()
                .ok_or_else(|| "car: empty list".to_string()),
            Value::Nil => Err("car: empty list".to_string()),
            _ => Err("car: argument must be a pair".to_string()),
        },
        PrimOp::Cdr => match &args[0] {
            Value::Pair(p) => Ok(p.1.clone()),
            Value::List(list) => match list.cdr() {
                Some(rest) if rest.is_empty() => Ok(Value::Nil),
                Some(rest) => Ok(Value::List(Arc::new(rest))),
                None => Err("cdr: empty list".to_string()),
            },
            Value::Nil => Err("cdr: empty list".to_string()),
            _ => Err("cdr: argument must be a pair".to_string()),
        },
        // `Value::Pair` is an immutable `Arc<(Value, Value)>` (unchanged by
        // this pass); mutating in place would need interior mutability
        // (`Rc<RefCell<...>>`) that Pair doesn't have. Rather than silently
        // no-op, surface a clear error - see final report.
        PrimOp::SetCar | PrimOp::SetCdr => Err(
            "set-car!/set-cdr!: mutable pairs are not supported by the interpreted backend"
                .to_string(),
        ),

        PrimOp::Eq => Ok(Value::Boolean(values_eqv(&args[0], &args[1]))),
        PrimOp::Eqv => Ok(Value::Boolean(values_eqv(&args[0], &args[1]))),

        PrimOp::Display => {
            print!("{}", args[0]);
            Ok(Value::Void)
        }
        PrimOp::Newline => {
            println!();
            Ok(Value::Void)
        }

        PrimOp::Not => Ok(Value::Boolean(!args[0].is_truthy())),

        PrimOp::List => Ok(vec_to_list(args)),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn session() -> Session {
        Session::new()
    }

    #[test]
    fn add_two_numbers() {
        let mut s = session();
        let r = call_prim(
            &PrimOp::Add,
            vec![Value::Number(Number::Int(1)), Value::Number(Number::Int(2))],
            &mut s,
        )
        .unwrap();
        assert_eq!(r, Value::Number(Number::Int(3)));
    }

    #[test]
    fn arity_mismatch_errors() {
        let mut s = session();
        let err = call_prim(&PrimOp::Add, vec![Value::Number(Number::Int(1))], &mut s).unwrap_err();
        assert!(err.contains("expected"));
    }

    #[test]
    fn identity_is_unary() {
        let mut s = session();
        let r = call_prim(&PrimOp::Identity, vec![Value::Boolean(true)], &mut s).unwrap();
        assert_eq!(r, Value::Boolean(true));
    }

    #[test]
    fn cons_car_cdr() {
        let mut s = session();
        let pair = call_prim(
            &PrimOp::Cons,
            vec![Value::Number(Number::Int(1)), Value::Number(Number::Int(2))],
            &mut s,
        )
        .unwrap();
        let car = call_prim(&PrimOp::Car, vec![pair.clone()], &mut s).unwrap();
        let cdr = call_prim(&PrimOp::Cdr, vec![pair], &mut s).unwrap();
        assert_eq!(car, Value::Number(Number::Int(1)));
        assert_eq!(cdr, Value::Number(Number::Int(2)));
    }

    #[test]
    fn list_is_variadic() {
        let mut s = session();
        let r = call_prim(&PrimOp::List, vec![], &mut s).unwrap();
        assert_eq!(r, Value::Nil);
    }
}
