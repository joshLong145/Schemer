use std::{
    collections::VecDeque,
    fmt::{self, Debug, Display, Formatter},
};

pub type RLispSymbol = String;

pub type RLispList = Vec<Atom>;
pub type RLispSubSymbolicExpressions = Vec<SymbolicExpression>;

pub type Tokens<'a> = &'a mut VecDeque<String>;

pub enum RLispNumber {
    Int(i32),
    Float(f32),
}

pub enum RLispBoolean {
    True(bool),
    False(bool),
}

pub enum Atom {
    Number(RLispNumber),
    Symbol(RLispSymbol),
    Bool(RLispBoolean),
}

pub type AtomToken = String;

#[derive(Clone, Eq, PartialEq)]
pub enum SymbolicExpression {
    Atom(AtomToken),
    List(RLispSubSymbolicExpressions),
}

impl TryFrom<SymbolicExpression> for AtomToken {
    type Error = &'static str;

    fn try_from(value: SymbolicExpression) -> Result<Self, Self::Error> {
        return match value {
            SymbolicExpression::Atom(exp) => Ok(exp),
            SymbolicExpression::List(_) => Err("Invalid cast atom"),
        };
    }
}

impl TryFrom<SymbolicExpression> for RLispSubSymbolicExpressions {
    type Error = &'static str;

    fn try_from(value: SymbolicExpression) -> Result<Self, Self::Error> {
        return match value {
            SymbolicExpression::Atom(_) => Err("Invalid cast list"),
            SymbolicExpression::List(l) => Ok(l),
        };
    }
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

impl Display for SymbolicExpression {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        match self {
            SymbolicExpression::Atom(atom_token) => write!(f, "{}", atom_token),
            SymbolicExpression::List(sub_exprs) => {
                write!(f, "(")?;
                for (i, expr) in sub_exprs.iter().enumerate() {
                    if i > 0 {
                        write!(f, " ")?;
                    }
                    write!(f, "{}", expr)?;
                }
                write!(f, ")")
            }
        }
    }
}

impl Debug for SymbolicExpression {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Self::Atom(arg0) => f.debug_tuple("Atom").field(arg0).finish(),
            Self::List(arg0) => f.debug_tuple("List").field(arg0).finish(),
        }
    }
}

impl SymbolicExpression {
    pub fn try_peek(&self) -> Option<SymbolicExpression> {
        match self {
            SymbolicExpression::Atom(_) => None,
            SymbolicExpression::List(vec) => Some(vec[0].clone()),
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
