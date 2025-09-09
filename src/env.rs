

use crate::{
    op::NumericOps, proc::Eval, types::{
        Atom, ExprKind, List, Quote, RLispBoolean, RLispNumber, SymbolicExpression
    }
};
use std::{collections::HashMap, sync::Arc};

pub fn std_const_exp() -> HashMap<String, ExprKind> {
    let mut const_exps = HashMap::new();
    const_exps.insert(
        "pi".to_string(),
        ExprKind::Atom(Arc::new(Atom::Number(RLispNumber::Float(3.14)))),
    );
    const_exps
}

type ProcedureFn = Box<dyn Fn(Vec<ExprKind>, &mut HashMap<String, ExprKind>) -> Result<ExprKind, String>>;

pub fn std_env() -> HashMap<String, ProcedureFn> {
    let mut env: HashMap<String, ProcedureFn> = HashMap::new();

    // Arithmetic operations
    env.insert(
        "+".to_string(),
        Box::new(|exp, _| -> Result<ExprKind, String> {
            if exp.len() < 2 {
                return Err("+ requires at least two arguments".to_string());
            }

            let l = match &exp[0] {
                ExprKind::Atom(a) => a.as_ref().clone(),
                _ => return Err("invalid expression".to_string()),
            };

            let r = match &exp[1] {
                ExprKind::Atom(a) => a.as_ref().clone(),
                _ => return Err("invalid expression".to_string()),
            };

            match l.add(r)? {
                SymbolicExpression::Atom(a) => Ok(ExprKind::Atom(Arc::new(Atom::from(a)))),
                _ => Err("invalid addition result".to_string()),
            }
        }),
    );

    env.insert(
        "-".to_string(),
        Box::new(|exp, _| -> Result<ExprKind, String> {
            if exp.len() < 2 {
                return Err("- requires at least two arguments".to_string());
            }

            let l = match &exp[0] {
                ExprKind::Atom(a) => a.as_ref().clone(),
                _ => return Err("invalid expression".to_string()),
            };

            let r = match &exp[1] {
                ExprKind::Atom(a) => a.as_ref().clone(),
                _ => return Err("invalid expression".to_string()),
            };

            match l.sub(r)? {
                SymbolicExpression::Atom(a) => Ok(ExprKind::Atom(Arc::new(Atom::from(a)))),
                _ => Err("invalid subtraction result".to_string()),
            }
        }),
    );

    env.insert(
        "*".to_string(),
        Box::new(|exp, _| -> Result<ExprKind, String> {
            if exp.len() < 2 {
                return Err("* requires at least two arguments".to_string());
            }

            let l = match &exp[0] {
                ExprKind::Atom(a) => a.as_ref().clone(),
                _ => return Err("invalid expression".to_string()),
            };

            let r = match &exp[1] {
                ExprKind::Atom(a) => a.as_ref().clone(),
                _ => return Err("invalid expression".to_string()),
            };

            match l.mul(r)? {
                SymbolicExpression::Atom(a) => Ok(ExprKind::Atom(Arc::new(Atom::from(a)))),
                _ => Err("invalid multiplication result".to_string()),
            }
        }),
    );

    env.insert(
        "/".to_string(),
        Box::new(|exp, _| -> Result<ExprKind, String> {
            if exp.len() < 2 {
                return Err("/ requires at least two arguments".to_string());
            }

            let l = match &exp[0] {
                ExprKind::Atom(a) => a.as_ref().clone(),
                _ => return Err("invalid expression".to_string()),
            };

            let r = match &exp[1] {
                ExprKind::Atom(a) => a.as_ref().clone(),
                _ => return Err("invalid expression".to_string()),
            };

            match l.div(r)? {
                SymbolicExpression::Atom(a) => Ok(ExprKind::Atom(Arc::new(Atom::from(a)))),
                _ => Err("invalid division result".to_string()),
            }
        }),
    );

    // List operations
    env.insert(
        "append".to_string(),
        Box::new(|exp, _| {
            if exp.len() < 2 {
                return Err("append requires at least two arguments".to_string());
            }

            let mut args = match &exp[0] {
                ExprKind::List(l) => l.args.clone(),
                _ => return Err("first argument must be a list".to_string()),
            };
            args.push(exp[1].clone());

            Ok(ExprKind::List(Arc::new(List {
                args,
                object_id: 0,
            })))
        }),
    );

    env.insert(
        "begin".to_string(),
        Box::new(|exp, _| {
            if exp.is_empty() {
                return Err("begin requires at least one expression".to_string());
            }
            Ok(exp[exp.len() - 1].clone())
        }),
    );

    // I/O operations
    env.insert(
        "display".to_string(),
        Box::new(|exp, _| {
            for expr in exp.iter() {
                println!("{}", expr);
            }
            Ok(ExprKind::Atom(Arc::new(Atom::Number(RLispNumber::Int(0)))))
        }),
    );

    // Type predicates
    env.insert(
        "number?".to_string(),
        Box::new(|exp, _| {
            if exp.is_empty() {
                return Err("number? requires one argument".to_string());
            }

            match &exp[0] {
                ExprKind::Atom(a) => match a.as_ref() {
                    Atom::Number(_) => Ok(ExprKind::Atom(Arc::new(Atom::Bool(RLispBoolean::True(true))))),
                    _ => Ok(ExprKind::Atom(Arc::new(Atom::Bool(RLispBoolean::False(false))))),
                },
                _ => Ok(ExprKind::Atom(Arc::new(Atom::Bool(RLispBoolean::False(false))))),
            }
        }),
    );

    env.insert(
        "list?".to_string(),
        Box::new(|exp, _| {
            if exp.is_empty() {
                return Err("list? requires one argument".to_string());
            }

            match &exp[0] {
                ExprKind::List(_) => Ok(ExprKind::Atom(Arc::new(Atom::Bool(RLispBoolean::True(true))))),
                _ => Ok(ExprKind::Atom(Arc::new(Atom::Bool(RLispBoolean::False(false))))),
            }
        }),
    );

    env.insert(
        "bool?".to_string(),
        Box::new(|exp, _| {
            if exp.is_empty() {
                return Err("bool? requires one argument".to_string());
            }

            match &exp[0] {
                ExprKind::Atom(a) => match a.as_ref() {
                    Atom::Bool(_) => Ok(ExprKind::Atom(Arc::new(Atom::Bool(RLispBoolean::True(true))))),
                    _ => Ok(ExprKind::Atom(Arc::new(Atom::Bool(RLispBoolean::False(false))))),
                },
                _ => Ok(ExprKind::Atom(Arc::new(Atom::Bool(RLispBoolean::False(false))))),
            }
        }),
    );

    // List operations
    env.insert(
        "len".to_string(),
        Box::new(|exp, _| {
            if exp.is_empty() {
                return Err("len requires one argument".to_string());
            }

            match &exp[0] {
                ExprKind::List(l) => Ok(ExprKind::Atom(Arc::new(Atom::Number(RLispNumber::Int(
                    l.args.len() as i32,
                ))))),
                _ => Err("argument must be a list".to_string()),
            }
        }),
    );

    env.insert(
        "car".to_string(),
        Box::new(|exp, _| {
            if exp.is_empty() {
                return Err("car requires one argument".to_string());
            }

            match &exp[0] {
                ExprKind::List(l) => {
                    if l.args.is_empty() {
                        Err("empty list".to_string())
                    } else {
                        Ok(l.args[0].clone())
                    }
                }
                _ => Err("argument must be a list".to_string()),
            }
        }),
    );

    env.insert(
        "cdr".to_string(),
        Box::new(|exp, _| {
            if exp.is_empty() {
                return Err("cdr requires one argument".to_string());
            }

            match &exp[0] {
                ExprKind::List(l) => {
                    if l.args.len() < 2 {
                        Err("list too short".to_string())
                    } else {
                        Ok(ExprKind::List(Arc::new(List {
                            args: l.args[1..].to_vec(),
                            object_id: 0,
                        })))
                    }
                }
                _ => Err("argument must be a list".to_string()),
            }
        }),
    );

    // Comparison operations
    env.insert(
        "<".to_string(),
        Box::new(|exp, _| -> Result<ExprKind, String> {
            if exp.len() < 2 {
                return Err("< requires at least two arguments".to_string());
            }

            let l = match &exp[0] {
                ExprKind::Atom(a) => a.as_ref().clone(),
                _ => return Err("invalid expression".to_string()),
            };

            let r = match &exp[1] {
                ExprKind::Atom(a) => a.as_ref().clone(),
                _ => return Err("invalid expression".to_string()),
            };

            Ok(ExprKind::Atom(Arc::new(Atom::Bool(if l < r {
                RLispBoolean::True(true)
            } else {
                RLispBoolean::False(false)
            }))))
        }),
    );

    env.insert(
        ">".to_string(),
        Box::new(|exp, _| -> Result<ExprKind, String> {
            if exp.len() < 2 {
                return Err("> requires at least two arguments".to_string());
            }

            let l = match &exp[0] {
                ExprKind::Atom(a) => a.as_ref().clone(),
                _ => return Err("invalid expression".to_string()),
            };

            let r = match &exp[1] {
                ExprKind::Atom(a) => a.as_ref().clone(),
                _ => return Err("invalid expression".to_string()),
            };

            Ok(ExprKind::Atom(Arc::new(Atom::Bool(if l > r {
                RLispBoolean::True(true)
            } else {
                RLispBoolean::False(false)
            }))))
        }),
    );

    env.insert(
        "=".to_string(),
        Box::new(|exp, _| -> Result<ExprKind, String> {
            if exp.len() < 2 {
                return Err("> requires at least two arguments".to_string());
            }

            let l = match &exp[0] {
                ExprKind::Atom(a) => a.as_ref().clone(),
                _ => return Err("invalid expression".to_string()),
            };

            let r = match &exp[1] {
                ExprKind::Atom(a) => a.as_ref().clone(),
                _ => return Err("invalid expression".to_string()),
            };

            Ok(ExprKind::Atom(Arc::new(Atom::Bool(if l == r {
                RLispBoolean::True(true)
            } else {
                RLispBoolean::False(false)
            }))))
        }),
    );

    env.insert(
        "and".to_string(),
        Box::new(|exp, _| -> Result<ExprKind, String> {
            if exp.len() < 2 {
                return Err("> requires at least two arguments".to_string());
            }

            let l = match &exp[0] {
                ExprKind::Atom(a) => {
                    match *a.to_owned() {
                        Atom::Bool(ref rlisp_boolean) => {
                            match rlisp_boolean {
                                RLispBoolean::True(_) => true,
                                RLispBoolean::False(_) => false,
                            }
                        },
                        _ => return Err("invalid expression".to_string()),
                    }
                },

                _ => return Err("invalid expression".to_string()),
            };

            let r = match &exp[1] {
                ExprKind::Atom(a) => {
                    match *a.to_owned() {
                        Atom::Bool(ref rlisp_boolean) => {
                            match rlisp_boolean {
                                RLispBoolean::True(_) => true,
                                RLispBoolean::False(_) => false,
                            }
                        },
                        _ => return Err("invalid expression".to_string()),
                    }
                },

                _ => return Err("invalid expression".to_string()),
            };

            Ok(ExprKind::Atom(Arc::new(Atom::Bool(if l && r {
                RLispBoolean::True(true)
            } else {
                RLispBoolean::False(false)
            }))))
        }),
    );

    env.insert(
        "or".to_string(),
        Box::new(|exp, _| -> Result<ExprKind, String> {
            if exp.len() < 2 {
                return Err("> requires at least two arguments".to_string());
            }

            let l = match &exp[0] {
                ExprKind::Atom(a) => {
                    match *a.to_owned() {
                        Atom::Bool(ref rlisp_boolean) => {
                            match rlisp_boolean {
                                RLispBoolean::True(_) => true,
                                RLispBoolean::False(_) => false,
                            }
                        },
                        _ => return Err("invalid expression".to_string()),
                    }
                },

                _ => return Err("invalid expression".to_string()),
            };

            let r = match &exp[1] {
                ExprKind::Atom(a) => {
                    match *a.to_owned() {
                        Atom::Bool(ref rlisp_boolean) => {
                            match rlisp_boolean {
                                RLispBoolean::True(_) => true,
                                RLispBoolean::False(_) => false,
                            }
                        },
                        _ => return Err("invalid expression".to_string()),
                    }
                },

                _ => return Err("invalid expression".to_string()),
            };

            Ok(ExprKind::Atom(Arc::new(Atom::Bool(if l || r {
                RLispBoolean::True(true)
            } else {
                RLispBoolean::False(false)
            }))))
        }),
    );

    env.insert(
        "not".to_string(),
        Box::new(|exp, _| -> Result<ExprKind, String> {
            if exp.len() < 1 {
                return Err("> requires at least two arguments".to_string());
            }

            let r = match &exp[0] {
                ExprKind::Atom(a) => {
                    match *a.to_owned() {
                        Atom::Bool(ref rlisp_boolean) => {
                            match rlisp_boolean {
                                RLispBoolean::True(_) => true,
                                RLispBoolean::False(_) => false,
                            }
                        },
                        _ => return Err("invalid expression".to_string()),
                    }
                },

                _ => return Err("invalid expression".to_string()),
            };

            Ok(ExprKind::Atom(Arc::new(Atom::Bool(if !r {
                RLispBoolean::True(true)
            } else {
                RLispBoolean::False(false)
            }))))
        }),
    );

    // List creation and manipulation
    env.insert(
        "list".to_string(),
        Box::new(|exp, _| {
            Ok(ExprKind::Quote(Arc::new(Quote {
                expr: ExprKind::List(Arc::new(List {
                    args: exp,
                    object_id: 0,
                })),
            })))
        }),
    );

    // Evaluation
    env.insert(
        "eval".to_string(),
        Box::new(|exp, symbol_defs| {
            if exp.is_empty() {
                return Err("eval requires one argument".to_string());
            }
            crate::eval::eval(exp[0].clone(), &std_env(), symbol_defs)
        }),
    );


    env.insert(
        "map".to_string(),
        Box::new(|exp, sd| {
            let env = std_env();

            let lambda_func = match &exp[0] {
                ExprKind::Lambda(lambda_exp) => lambda_exp.clone(),
                _ => return Err("First argument to map must be a lambda".to_string()),
            };

            let args = match &exp[1] {
                ExprKind::Quote(args) => {
                    match args.expr.clone() {
                        ExprKind::List(l) => {
                            Arc::try_unwrap(l).unwrap_or_else(|arc| (*arc).clone())
                        },
                        _ => {
                            return Err("Invalid symbolic expression".to_string());
                        }
                    }
                }
                _ => return Err(format!("Second argument to map must be a list expression {:?}", exp[1])),
            };

            let mut mapping_results: Vec<ExprKind> = Vec::new();

            for arg in args.args.clone() {
                let proc = ExprKind::to_proc(
                    &ExprKind::Lambda(lambda_func.clone()),
                    ExprKind::List(Arc::new(List{
                        args: vec![arg],
                        object_id: 0,
                    })),
                    &env).unwrap();
                let result = proc.proc_eval(sd).unwrap();
                mapping_results.push(result);
            }

            Ok(ExprKind::Quote(Arc::new(Quote{
                expr: ExprKind::List(Arc::new(List {
                    args: mapping_results,
                    object_id: 0,
                })),
            })))
        }),
    );


    env.insert("filter".to_string(),
        Box::new(|exp, sd| {
            let env = std_env();

            let lambda_func = match &exp[0] {
                ExprKind::Lambda(lambda_exp) => lambda_exp.clone(),
                _ => return Err("First argument to map must be a lambda".to_string()),
            };

            let args = match &exp[1] {
                ExprKind::Quote(args) => {
                    match args.expr.clone() {
                        ExprKind::List(l) => {
                            Arc::try_unwrap(l).unwrap_or_else(|arc| (*arc).clone())
                        },
                        _ => {
                            return Err("Invalid symbolic expression".to_string());
                        }
                    }
                }
                _ => return Err(format!("Second argument to map must be a list expression {:?}", exp[1])),
            };

            let mut mapping_results: Vec<ExprKind> = Vec::new();

            for arg in args.args.iter() {
                let proc = ExprKind::to_proc(
                    &ExprKind::Lambda(lambda_func.clone()),
                    ExprKind::List(Arc::new(List{
                        args: vec![arg.clone()],
                        object_id: 0,
                    })),
                    &env).unwrap();
                let result = proc.proc_eval(sd).unwrap();
                match result {
                    ExprKind::Atom(ref atom) => {
                        if let Atom::Bool(test) = atom.as_ref() {
                            if let RLispBoolean::True(_) = test {
                                mapping_results.push(arg.clone());
                            }
                        }
                    },

                    _ => {
                        return Err("Invalid return from filter predicate".to_string());
                    }
                }
            }

            Ok(ExprKind::Quote(Arc::new(Quote{
                expr: ExprKind::List(Arc::new(List {
                    args: mapping_results,
                    object_id: 0,
                })),
            })))
        })
    );

    env
}
