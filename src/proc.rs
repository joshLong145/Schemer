use std::collections::HashMap;
use std::fmt::{self, Display, Formatter};

#[cfg(test)]
use std::sync::Arc;

use crate::{
    eval::eval,
    types::{Atom, ExprKind},
};

pub type ProcedureFn =
    Box<dyn Fn(Vec<ExprKind>, &mut HashMap<String, ExprKind>) -> Result<ExprKind, String>>;

pub struct Proc<'a> {
    pub params: HashMap<String, ExprKind>,
    pub signature: ExprKind,
    pub body: ExprKind,
    pub env: &'a HashMap<String, ProcedureFn>,
}

impl Display for Proc<'_> {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "\nsignature {}\nparameters {}\nbody {}",
            self.signature,
            self.params
                .keys()
                .fold(String::new(), |acc, key| acc + " " + key),
            self.body
        )
    }
}

pub trait Eval {
    fn proc_eval(
        &self,
        symbol_definitions: &mut HashMap<String, ExprKind>,
    ) -> Result<ExprKind, String>;
}

impl Eval for Proc<'_> {
    fn proc_eval(
        &self,
        symbol_definitions: &mut HashMap<String, ExprKind>,
    ) -> Result<ExprKind, String> {
        let mut local_symbols = HashMap::new();
        local_symbols.extend(symbol_definitions.clone());
        local_symbols.extend(self.params.clone());

        match &self.signature {
            ExprKind::List(list) => {
                for param in list.args.iter() {
                    match param {
                        ExprKind::Atom(atom) => match atom.as_ref() {
                            Atom::Symbol(s) => {
                                if !local_symbols.contains_key(s) {
                                    return Err(format!("unbound parameter: {}", s));
                                }
                            }
                            _ => return Err("invalid parameter type".to_string()),
                        },
                        _ => return Err("parameters must be symbols".to_string()),
                    }
                }
            }
            ExprKind::Atom(atom) => match atom.as_ref() {
                Atom::Symbol(s) => {
                    if !local_symbols.contains_key(s) {
                        return Err(format!("unbound parameter: {}", s));
                    }
                }
                _ => return Err("invalid parameter type".to_string()),
            },
            _ => return Err("invalid parameter specification".to_string()),
        }

        eval(self.body.clone(), self.env, &mut local_symbols)
    }
}

#[cfg(test)]
mod tests {
    use crate::types::{List, RLispNumber};

    use super::*;

    #[test]
    fn test_proc_eval_simple() {
        let env = HashMap::new();
        let mut symbol_defs = HashMap::new();
        let mut params = HashMap::new();

        // Create a simple procedure that returns a number
        params.insert(
            "x".to_string(),
            ExprKind::Atom(Arc::new(Atom::Number(RLispNumber::Int(42)))),
        );

        let proc = Proc {
            params,
            signature: ExprKind::Atom(Arc::new(Atom::Symbol("x".to_string()))),
            body: ExprKind::Atom(Arc::new(Atom::Symbol("x".to_string()))),
            env: &env,
        };

        let result = proc.proc_eval(&mut symbol_defs).unwrap();
        match result {
            ExprKind::Atom(atom) => match atom.as_ref() {
                Atom::Number(RLispNumber::Int(n)) => assert_eq!(*n, 42),
                _ => panic!("expected integer"),
            },
            _ => panic!("expected atom"),
        }
    }

    #[test]
    fn test_proc_eval_with_closure() {
        let env = HashMap::new();
        let mut symbol_defs = HashMap::new();
        let mut params = HashMap::new();

        // Add a value to the outer scope
        symbol_defs.insert(
            "y".to_string(),
            ExprKind::Atom(Arc::new(Atom::Number(RLispNumber::Int(10)))),
        );

        // Create a procedure that references both a parameter and a closed-over value
        params.insert(
            "x".to_string(),
            ExprKind::Atom(Arc::new(Atom::Number(RLispNumber::Int(5)))),
        );

        let proc = Proc {
            params,
            signature: ExprKind::Atom(Arc::new(Atom::Symbol("x".to_string()))),
            body: ExprKind::List(Arc::new(List {
                args: vec![
                    ExprKind::Atom(Arc::new(Atom::Symbol("x".to_string()))),
                    ExprKind::Atom(Arc::new(Atom::Symbol("y".to_string()))),
                ],
                object_id: 0,
            })),
            env: &env,
        };

        let result = proc.proc_eval(&mut symbol_defs).unwrap();
        match result {
            ExprKind::List(list) => {
                assert_eq!(list.args.len(), 2);
                match &list.args[0] {
                    ExprKind::Atom(atom) => match atom.as_ref() {
                        Atom::Number(RLispNumber::Int(n)) => assert_eq!(*n, 5),
                        _ => panic!("expected integer"),
                    },
                    _ => panic!("expected atom"),
                }
                match &list.args[1] {
                    ExprKind::Atom(atom) => match atom.as_ref() {
                        Atom::Number(RLispNumber::Int(n)) => assert_eq!(*n, 10),
                        _ => panic!("expected integer"),
                    },
                    _ => panic!("expected atom"),
                }
            }
            _ => panic!("expected list"),
        }
    }
}
