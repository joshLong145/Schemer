use std::{
    collections::{HashMap, VecDeque},
    fmt::{self, Debug, Display, Formatter},
    sync::Arc,
};

use log::debug;

use crate::proc::{Proc, ProcedureFn};

pub type RLispSymbol = String;

pub type RLispList<'a> = Vec<Atom>;
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
    ListExpr(RLispSubSymbolicExpressions),
    Lambda(RLispSubSymbolicExpressions),
}

#[derive(Clone, Debug, PartialEq)]
pub enum ExprKind {
    If(Arc<If>),
    Define(Arc<Define>),
    Begin(Arc<Begin>),
    Lambda(Arc<Lambda>),
    Atom(Arc<Atom>),
    List(Arc<List>),
    Quote(Arc<Quote>),
}

#[derive(Clone, Debug, PartialEq)]
pub struct If {
    pub test_expr: ExprKind,
    pub then_expr: ExprKind,
    pub else_expr: ExprKind,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Begin {
    pub exprs: Vec<ExprKind>
}

#[derive(Clone, Debug, PartialEq)]
pub struct Define {
    pub name: ExprKind,
    pub body: ExprKind,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Lambda {
    pub args: ExprKind,
    pub body: ExprKind,
    pub object_id: u64,
}

#[derive(Clone, Debug, PartialEq)]
pub struct List {
    pub args: Vec<ExprKind>,
    pub object_id: u64
}

#[derive(Clone, Debug, PartialEq)]
pub struct Quote{
    pub expr: ExprKind
}

impl Clone for RLispBoolean {
    fn clone(&self) -> Self {
        match self {
            Self::True(arg0) => Self::True(arg0.clone()),
            Self::False(arg0) => Self::False(arg0.clone()),
        }
    }
}

impl Clone for RLispNumber {
    fn clone(&self) -> Self {
        match self {
            Self::Int(arg0) => Self::Int(arg0.clone()),
            Self::Float(arg0) => Self::Float(arg0.clone()),
        }
    }
}

impl Clone for Atom {
    fn clone(&self) -> Self {
        match self {
            Self::Number(arg0) => Self::Number(arg0.clone()),
            Self::Symbol(arg0) => Self::Symbol(arg0.clone()),
            Self::Bool(arg0) => Self::Bool(arg0.clone()),
        }
    }
}


impl TryFrom<SymbolicExpression> for AtomToken {
    type Error = &'static str;

    fn try_from(value: SymbolicExpression) -> Result<Self, Self::Error> {
        return match value {
            SymbolicExpression::Atom(exp) => Ok(exp),
            _ => Err("Invalid cast"),

        };
    }
}

impl TryFrom<SymbolicExpression> for RLispSubSymbolicExpressions {
    type Error = &'static str;

    fn try_from(value: SymbolicExpression) -> Result<Self, Self::Error> {
        return match value {
            SymbolicExpression::List(l) => Ok(l),
            SymbolicExpression::Lambda(la) => Ok(la),
            _ => Err("Invalid casta"),
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
            RLispBoolean::True(_) => write!(f, "#t"),
            RLispBoolean::False(_) => write!(f, "#f"),
        }
    }
}

impl Display for Atom {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        match self {
            Atom::Number(n) => write!(f, "{}", n),
            Atom::Symbol(s) => write!(f, "{}", s),
            Atom::Bool(b) => {
                match b {
                    RLispBoolean::True(_) => write!(f, "#t"),
                    RLispBoolean::False(_) => write!(f, "#f"),
                }
            },
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
            SymbolicExpression::ListExpr(sub_exprs) => {
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

impl Display for ExprKind {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match self {
            ExprKind::If(if_expr) => {
                write!(f, "(\nif \n{} \n{} \n{}\n)", if_expr.test_expr, if_expr.then_expr, if_expr.else_expr)
            }
            ExprKind::Define(define_expr) => {
                write!(f, "\n(\ndefine {}\n\t{}\n)", define_expr.name, define_expr.body)
            }
            ExprKind::Begin(begin_expr) => {
                write!(f, "\n(\nbegin")?;
                for expr in begin_expr.exprs.iter() {
                    write!(f, "\n{}\n", expr)?;
                }
                write!(f, ")")
            }
            ExprKind::Lambda(lambda_expr) => {
                write!(f, "(\nlambda\n{}\n{}\n)", lambda_expr.args, lambda_expr.body)
            }
            ExprKind::Atom(atom_expr) => {
                write!(f, "{}", atom_expr)
            }
            ExprKind::List(list_expr) => {
                write!(f, "(")?;
                for (i, expr) in list_expr.args.iter().enumerate() {
                    if i > 0 {
                        write!(f, " ")?;
                    }
                    write!(f, "{}", expr)?;
                }
                write!(f, ")")
            }
            ExprKind::Quote(quote_expr) => {
                write!(f, "'{}", quote_expr.expr)
            }
        }
    }
}

impl Debug for RLispNumber {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Self::Int(arg0) => f.debug_tuple("Int").field(arg0).finish(),
            Self::Float(arg0) => f.debug_tuple("Float").field(arg0).finish(),
        }
    }
}

impl Debug for RLispBoolean {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Self::True(arg0) => f.debug_tuple("#t").field(arg0).finish(),
            Self::False(arg0) => f.debug_tuple("#f").field(arg0).finish(),
        }
    }
}

impl Debug for Atom {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Self::Number(arg0) => f.debug_tuple("Number").field(arg0).finish(),
            Self::Symbol(arg0) => f.debug_tuple("Symbol").field(arg0).finish(),
            Self::Bool(arg0) => f.debug_tuple("Bool").field(arg0).finish(),
        }
    }
}


impl Debug for SymbolicExpression {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Self::Atom(arg0) => f.debug_tuple("Atom").field(arg0).finish(),
            Self::List(arg0) => f.debug_tuple("List").field(arg0).finish(),
            Self::ListExpr(arg0) => f.debug_tuple("List").field(arg0).finish(),
            Self::Lambda(arg0) => f.debug_tuple("Lambda").field(arg0).finish(),
        }
    }
}

impl SymbolicExpression {
    pub fn try_peek(&self) -> Option<SymbolicExpression> {
        match self {
            SymbolicExpression::Atom(_) => None,
            SymbolicExpression::List(vec) => Some(vec[0].clone()),
            SymbolicExpression::ListExpr(vec) => Some(vec[0].clone()),
            SymbolicExpression::Lambda(la) => Some(la[0].clone()),
        }
    }
}

impl From<&String> for Atom {
    fn from(value: &String) -> Self {
        if let Ok(a) = value.parse::<i32>() {
            Atom::Number(RLispNumber::Int(a))
        } else if let Ok(a) = value.parse::<f32>() {
            Atom::Number(RLispNumber::Float(a))
        } else if value == "#t" {
            Atom::Bool(RLispBoolean::True(true))
        } else if value == "#f" {
            Atom::Bool(RLispBoolean::False(false))
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
        } else if value == "#t" {
            Atom::Bool(RLispBoolean::True(true))
        } else if value == "#f" {
            Atom::Bool(RLispBoolean::False(false))
        } else {
            Atom::Symbol(value.clone())
        }
    }
}

impl ExprKind {
    pub fn to_proc<'a>(
            &self,
            params: ExprKind,
            env: &'a HashMap<String, ProcedureFn>,
        ) -> Result<Proc<'a>, String> {
            match self {
                ExprKind::Lambda(lambda) => {
                    let mut param_map = HashMap::new();
                    match params {
                        ExprKind::List(l) => {
                            // Extract parameter names from lambda args
                            match &lambda.args {
                                ExprKind::List(param_list) => {
                                    debug!("mapping parameters to signature {:?} args {:?}", param_list, l);
                                    for i in 0..param_list.args.len() {
                                        if let ExprKind::Atom(atom) = param_list.args[i].clone() {
                                            if let Atom::Symbol(ref name) = *atom {
                                                param_map.insert(name.clone(), l.args[i].clone());
                                            } else {
                                                return Err("lambda parameters must be symbols".to_string());
                                            }
                                        } else {
                                            return Err("lambda parameters must be symbols".to_string());
                                        }
                                    }
                                }
                                ExprKind::Atom(atom) => {
                                    if let Atom::Symbol(ref name) = **atom {
                                        param_map.insert(name.clone(), lambda.args.clone());
                                    } else {
                                        return Err("lambda parameter must be a symbol".to_string());
                                    }
                                }
                                _ => return Err("invalid lambda parameter specification".to_string()),
                            }

                            return Ok(Proc {
                                params: param_map.clone(),
                                signature: lambda.args.clone(),
                                body: lambda.body.clone(),
                                env: env,
                            });
                        }
                        ExprKind::Atom(a) => {
                            // Extract parameter names from lambda args
                            match &lambda.args {
                                ExprKind::List(param_list) => {
                                    debug!("mapping parameters to signature {:?} args {:?}", param_list, a);
                                    for i in 0..param_list.args.len() {
                                        if let ExprKind::Atom(atom) = param_list.args[i].clone() {
                                            if let Atom::Symbol(ref name) = *atom {
                                                param_map.insert(name.clone(), ExprKind::Atom(a.clone()));
                                            } else {
                                                return Err("lambda parameters must be symbols".to_string());
                                            }
                                        } else {
                                            return Err("lambda parameters must be symbols".to_string());
                                        }
                                    }
                                }
                                ExprKind::Atom(atom) => {
                                    if let Atom::Symbol(ref name) = **atom {
                                        param_map.insert(name.clone(), lambda.args.clone());
                                    } else {
                                        return Err("lambda parameter must be a symbol".to_string());
                                    }
                                }
                                _ => return Err("invalid lambda parameter specification".to_string()),
                            }

                            return Ok(Proc {
                                params: param_map.clone(),
                                signature: lambda.args.clone(),
                                body: lambda.body.clone(),
                                env: env,
                            });
                        }
                        _ =>  Err("invalid expression".to_string())
                    }

                }
                _ => Err("can only create procedures from lambda expressions".to_string()),
            }
        }

    pub fn is_proc(&self) -> bool {
        matches!(self, ExprKind::Lambda(_))
    }
}


impl From<SymbolicExpression> for ExprKind {
    fn from(value: SymbolicExpression) -> Self {
            match value {
                SymbolicExpression::Atom(atom) => ExprKind::Atom(Arc::new(Atom::from(atom))),
                SymbolicExpression::List(symbolic_expressions) => {
                    let first = &symbolic_expressions[0];
                    match first {
                        SymbolicExpression::Atom(name) => {
                            if name == "if" {
                                ExprKind::If(Arc::new(If {
                                    test_expr: symbolic_expressions[1].clone().into(),
                                    then_expr: symbolic_expressions[2].clone().into(),
                                    else_expr: symbolic_expressions[3].clone().into(),
                                }))
                            } else if name == "begin" {
                              ExprKind::Begin(Arc::new(Begin {
                                  exprs: symbolic_expressions[1..symbolic_expressions.len()]
                                      .into_iter()
                                      .map(|exp| ExprKind::from(exp.clone()))
                                      .collect()
                              }))
                            } else if name == "define" {
                                ExprKind::Define(Arc::new(Define {
                                    name: symbolic_expressions[1].clone().into(),
                                    body: symbolic_expressions[2].clone().into(),
                                }))
                            } else {
                                ExprKind::List(Arc::new(List {
                                    args: symbolic_expressions.into_iter().map(ExprKind::from).collect(),
                                    object_id: 0
                                }))
                            }
                        },
                        _ => {
                            ExprKind::List(Arc::new(List {
                                args: symbolic_expressions.into_iter().map(ExprKind::from).collect(),
                                object_id: 0,
                            }))
                        },
                    }
                },
                SymbolicExpression::ListExpr(symbolic_expressions) => ExprKind::Quote(Arc::new(Quote {
                    expr: ExprKind::List(Arc::new(List {
                        args: symbolic_expressions.into_iter().map(ExprKind::from).collect(),
                        object_id: 0,
                    }))
                })),
                SymbolicExpression::Lambda(symbolic_expressions) => ExprKind::Lambda(Arc::new(Lambda {
                    args: symbolic_expressions[1].clone().into(),
                    body: symbolic_expressions[2].clone().into(),
                    object_id: 0,
                })),
            }
        }
}
