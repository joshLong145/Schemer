use std::{
    collections::HashMap,
    fmt::{self, Display, Formatter},
    sync::Arc,
};

use crate::types::ExprKind;

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
    /// Body expression (unevaluated)
    pub body: ExprKind,
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
    /// Empty list
    Nil,
    /// Symbol (for quoted symbols)
    Symbol(String),
    /// Void (result of define, set!, etc.)
    Void,
}

impl Value {
    /// R7RS truthiness: only #f is falsy
    pub fn is_truthy(&self) -> bool {
        !matches!(self, Value::Boolean(false))
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
            Value::Nil => write!(f, "()"),
            Value::Symbol(s) => write!(f, "{}", s),
            Value::Void => write!(f, "#<void>"),
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
