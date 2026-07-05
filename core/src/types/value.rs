use std::{
    cell::RefCell,
    collections::HashMap,
    fmt::{self, Display, Formatter},
    rc::Rc,
    sync::Arc,
};

use crate::types::env::Closure;
use crate::types::list::SchemeList;

/// Scheme number representation (simple tower)
#[derive(Clone, Debug, PartialEq)]
pub enum Number {
    /// Exact integer
    Int(i64),
    /// Inexact floating-point
    Float(f64),
}

impl Display for Number {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Number::Int(i) => write!(f, "{}", i),
            Number::Float(fl) => write!(f, "{}", fl),
        }
    }
}

/// A Scheme procedure (closure)
#[derive(Clone, Debug)]
pub struct Procedure {
    /// Parameter names
    pub params: Vec<String>,
    /// Body expression (unevaluated) - stored as Value for homoiconicity
    pub body: Box<Value>,
    /// Captured environment at closure creation
    pub env: HashMap<String, Value>,
}

impl PartialEq for Procedure {
    fn eq(&self, other: &Self) -> bool {
        // Procedures are equal only if they are the same closure
        std::ptr::eq(self, other)
    }
}

/// Runtime values in Scheme (R7RS-small)
#[derive(Clone, Debug, PartialEq)]
pub enum Value {
    /// Numeric value (exact or inexact)
    Number(Number),
    /// Boolean (#t or #f)
    Boolean(bool),
    /// String
    String(Arc<String>),
    /// Character
    Char(char),
    /// Procedure (evaluated lambda)
    Procedure(Arc<Procedure>),
    /// Pair/cons cell
    Pair(Arc<(Value, Value)>),
    /// Proper list (nil-terminated chain of pairs)
    List(Arc<SchemeList>),
    /// Empty list
    Nil,
    /// Symbol (for quoted symbols)
    Symbol(String),
    /// Void (result of define, set!, etc.)
    Void,
    /// Mutable cell, used by the ANF interpreter for `set!`/mutable captured
    /// variables (`MakeBox`/`ReadBox`/`WriteBox`). The compiled runtime has
    /// an equivalent (`Box` in `runtime_types`); this fills the same gap for
    /// the tree-walking interpreter.
    Box(Rc<RefCell<Value>>),
    /// A closure produced by the ANF interpreter (`ComplexExpr::MakeClosure`):
    /// a function label plus the lexical environment captured at creation.
    /// Distinct from `Procedure` (which belongs to the older Value-walking
    /// evaluator, `core/src/eval.rs`) since the two closures have
    /// incompatible internal shapes (label+env-chain vs body+env-map).
    Closure(Closure),
}

impl Value {
    /// R7RS truthiness: only #f is falsy
    pub fn is_truthy(&self) -> bool {
        !matches!(self, Value::Boolean(false))
    }

    /// Extract a `Closure` from a `Value`, for use as the callee of an
    /// application. Errors with a descriptive message otherwise.
    pub fn expect_closure(&self) -> Result<Closure, String> {
        match self {
            Value::Closure(c) => Ok(c.clone()),
            other => Err(format!("not a procedure: {}", other)),
        }
    }
}

impl Display for Value {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Value::Number(n) => write!(f, "{}", n),
            Value::Boolean(true) => write!(f, "#t"),
            Value::Boolean(false) => write!(f, "#f"),
            Value::String(s) => write!(f, "\"{}\"", s),
            Value::Char(c) => write!(f, "#\\{}", c),
            Value::Procedure(_) => write!(f, "#<procedure>"),
            Value::Pair(p) => {
                write!(f, "(")?;
                write_pair(f, &p.0, &p.1)
            }
            Value::List(list) => write!(f, "{}", list),
            Value::Nil => write!(f, "()"),
            Value::Symbol(s) => write!(f, "{}", s),
            Value::Void => write!(f, "#<void>"),
            Value::Box(_) => write!(f, "#<box>"),
            Value::Closure(_) => write!(f, "#<closure>"),
        }
    }
}

/// Helper to write pairs in proper list notation when possible
fn write_pair(f: &mut Formatter<'_>, car: &Value, cdr: &Value) -> fmt::Result {
    write!(f, "{}", car)?;
    match cdr {
        Value::Nil => write!(f, ")"),
        Value::Pair(p) => {
            write!(f, " ")?;
            write_pair(f, &p.0, &p.1)
        }
        other => write!(f, " . {})", other),
    }
}
