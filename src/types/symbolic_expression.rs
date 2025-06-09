use std::{
    collections::{HashMap},
    fmt::{self, Debug, Display, Formatter},
};

use crate::proc::Proc;

pub type RLispSubSymbolicExpressions = Vec<SymbolicExpression>;
pub type AtomToken = String;

#[derive(Clone, Eq, PartialEq)]
pub enum SymbolicExpression {
    Atom(AtomToken),
    List(RLispSubSymbolicExpressions),
    Lambda(RLispSubSymbolicExpressions),
}

impl TryFrom<SymbolicExpression> for AtomToken {
    type Error = &'static str;

    fn try_from(value: SymbolicExpression) -> Result<Self, Self::Error> {
        return match value {
            SymbolicExpression::Atom(exp) => Ok(exp),
            SymbolicExpression::List(_) => Err("Invalid cast atom"),
            SymbolicExpression::Lambda(_) => Err("Invalid cast atom"),
        };
    }
}

impl TryFrom<SymbolicExpression> for RLispSubSymbolicExpressions {
    type Error = &'static str;

    fn try_from(value: SymbolicExpression) -> Result<Self, Self::Error> {
        return match value {
            SymbolicExpression::Atom(_) => Err("Invalid cast list"),
            SymbolicExpression::List(l) => Ok(l),
            SymbolicExpression::Lambda(la) => Ok(la),
        };
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
            SymbolicExpression::Lambda(sub_exprs) => {
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
            Self::Lambda(arg0) => f.debug_tuple("Lambda").field(arg0).finish(),
        }
    }
}

impl SymbolicExpression {
    pub fn try_peek(&self) -> Option<SymbolicExpression> {
        match self {
            SymbolicExpression::Atom(_) => None,
            SymbolicExpression::List(vec) => Some(vec[0].clone()),
            SymbolicExpression::Lambda(la) => Some(la[0].clone()),
        }
    }
}
 

impl SymbolicExpression {
    pub fn to_proc(
        value: Vec<SymbolicExpression>,
        env: &HashMap<
            String,
            Box<dyn Fn(RLispSubSymbolicExpressions) -> Result<SymbolicExpression, String>>,
        >,
    ) -> Result<Proc, String> {
        let exp = value[0].clone();
        match exp {
            SymbolicExpression::Atom(exp) => {
                if exp == "lambda" {
                    let body = value[2].clone();
                    let signature = value[1].clone();
                    let param_map: &mut HashMap<String, SymbolicExpression> = &mut HashMap::new();

                    let proc = Proc {
                        params: param_map.clone(),
                        body: body.clone(),
                        env,
                        signature,
                    };

                    return Ok(proc);
                }
                Err("".to_string())
            }
            SymbolicExpression::List(_) => {
                return Err("".to_string());
            }
            SymbolicExpression::Lambda(la) => {
                let body = value[2].clone();
                let param_map: &mut HashMap<String, SymbolicExpression> = &mut HashMap::new();

                let proc = Proc {
                    params: param_map.clone(),
                    body: body.clone(),
                    env: std::rc::Rc::new(std::cell::RefCell::new(*env)),
                    signature: la[1].clone(),
                };

                return Ok(proc);
            }
        }
    }

    pub fn is_proc(exp: &SymbolicExpression) -> bool {
        if let Some(p) = exp.try_peek() {
            if let Some(q) = p.try_peek() {
                if q.to_string() == "lambda" {
                    return true;
                }
            }
        }

        false
    }
}



