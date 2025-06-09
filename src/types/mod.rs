use std::collections::VecDeque;

mod atom;
mod symbolic_expression;
mod value;
mod list;

pub use atom::{Atom, RLispNumber, RLispBoolean};
pub use symbolic_expression::{SymbolicExpression, RLispSubSymbolicExpressions, AtomToken};
pub type Tokens<'a> = &'a mut VecDeque<String>;
pub type RLispList<'a> = Vec<Atom>;
pub type RLispSymbol = String;


