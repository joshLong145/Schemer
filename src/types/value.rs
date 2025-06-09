
use crate::proc::Proc;

use super::{list::List, RLispNumber};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Symbol(pub String);

impl std::fmt::Display for Symbol {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

#[derive(Debug, Clone)]
pub enum Value {
    True,
    False,
    Number(RLispNumber),
    Lambda(Proc),
    Symbol(Symbol),
    List(List),
}

impl std::hash::Hash for Value {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        core::mem::discriminant(self).hash(state);
    }
}


impl std::fmt::Display for Value {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            Value::True => write!(f, "{}", true),
            Value::False => write!(f, "{}", false),
            Value::Number(rlisp_number) => write!(f, "{}", rlisp_number),
            Value::Lambda(proc) => write!(f, "{}", proc),
            Value::Symbol(symbol) => write!(f, "{}", symbol),
            Value::List(list) => write!(f, "{}", list),
        }
    }
}


impl PartialEq for Value {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Value::True, Value::True) => true,
            (Value::List(this), Value::List(other)) => this == other,
            (Value::Number(this), Value::Number(other)) => this == other,
            (Value::Symbol(this), Value::Symbol(other)) => this == other,
            _ => false,
        }
    }
}

impl Eq for Value {}
