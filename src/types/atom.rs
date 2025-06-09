use std::fmt::{self, Display, Formatter};

use super::RLispSymbol;

#[derive(Debug, Clone)]
pub enum RLispNumber {
    Int(i32),
    Float(f32),
}

#[derive(Clone)]
pub enum RLispBoolean {
    True(bool),
    False(bool),
}

#[derive(Clone)]
pub enum Atom {
    Number(RLispNumber),
    Symbol(RLispSymbol),
    Bool(RLispBoolean),
}

impl Display for RLispNumber {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        match self {
            RLispNumber::Int(i) => write!(f, "{}", i),
            RLispNumber::Float(fl) => write!(f, "{}", fl),
        }
    }
}

impl Display for RLispBoolean {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        match self {
            RLispBoolean::True(b) => write!(f, "{}", b),
            RLispBoolean::False(b) => write!(f, "{}", b),
        }
    }
}

impl Display for Atom {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        match self {
            Atom::Number(n) => write!(f, "{}", n),
            Atom::Symbol(s) => write!(f, "{}", s),
            Atom::Bool(b) => write!(f, "{}", b),
        }
    }
}

impl From<&String> for Atom {
    fn from(value: &String) -> Self {
        if let Ok(a) = value.parse::<i32>() {
            Atom::Number(RLispNumber::Int(a))
        } else if let Ok(a) = value.parse::<f32>() {
            Atom::Number(RLispNumber::Float(a))
        } else if let Ok(a) = value.parse::<bool>() {
            if a {
                Atom::Bool(RLispBoolean::True(a))
            } else {
                Atom::Bool(RLispBoolean::False(a))
            }
        } else {
            Atom::Symbol(value.clone())
        }
    }
}

impl From<String> for Atom {
    fn from(value: String) -> Self {
        if let Ok(a) = value.parse::<i32>() {
            Atom::Number(RLispNumber::Int(a))
        } else if let Ok(a) = value.parse::<f32>() {
            Atom::Number(RLispNumber::Float(a))
        } else if let Ok(a) = value.parse::<bool>() {
            if a {
                Atom::Bool(RLispBoolean::True(a))
            } else {
                Atom::Bool(RLispBoolean::False(a))
            }
        } else {
            Atom::Symbol(value.clone())
        }
    }
}
