use log::debug;

use crate::{
    op::NumericOps,
    proc::Eval,
    types::{
        pair::Pair, list::PairList, Atom, ExprKind, Quote, RLispBoolean, RLispNumber, SymbolicExpression,
    },
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

type ProcedureFn =
    Box<dyn Fn(Vec<ExprKind>, &mut HashMap<String, ExprKind>) -> Result<ExprKind, String>>;

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

            let mut agg: Vec<ExprKind> = vec![];

            // Helper function to extract list from either quoted or unquoted list
            let extract_list = |expr: &ExprKind| -> Result<Vec<ExprKind>, String> {
                match expr {
                    ExprKind::List(l) => Ok(l.to_vec()),
                    ExprKind::Quote(q) => {
                        if let ExprKind::List(l) = q.expr.clone() {
                            Ok(l.to_vec())
                        } else {
                            Err("argument must be a list".to_string())
                        }
                    }
                    _ => Err("argument must be a list".to_string()),
                }
            };

            // Process all arguments (including first one)
            for e in exp.iter() {
                let list_elements = extract_list(e)?;
                agg.extend(list_elements);
            }

            Ok(ExprKind::Quote(Arc::new(Quote {
                expr: ExprKind::List(Arc::new(PairList::from_vec(agg))),
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

    env.insert(
        "cons".to_string(),
        Box::new(|exp, _| {
            if exp.len() != 2 {
                return Err("cons requires 2 arguments".to_string());
            }
            let car = exp[0].clone();
            let cdr_expr = exp[1].clone();

            // Helper function to convert List/Quote(List) to Pair chain
            let convert_to_cdr = |expr: ExprKind| -> Option<Arc<ExprKind>> {
                match expr {
                    // If cdr is a List, extract its Pair chain
                    ExprKind::List(list) => {
                        if let Some(pair_head) = &list.head {
                            Some(Arc::new(ExprKind::Pair(pair_head.clone())))
                        } else {
                            // Empty list - proper nil terminator
                            None
                        }
                    },
                    // If cdr is a quoted List, extract its Pair chain
                    ExprKind::Quote(q) => {
                        if let ExprKind::List(list) = q.as_ref().expr.clone() {
                            if let Some(pair_head) = &list.head {
                                Some(Arc::new(ExprKind::Pair(pair_head.clone())))
                            } else {
                                None
                            }
                        } else {
                            // Quoted non-list, wrap it
                            Some(Arc::new(ExprKind::Quote(q)))
                        }
                    },
                    // If cdr is already a Pair, use it as-is
                    ExprKind::Pair(p) => Some(Arc::new(ExprKind::Pair(p))),
                    // For any other type, create improper list (dotted pair)
                    other => Some(Arc::new(other)),
                }
            };

            let cdr = convert_to_cdr(cdr_expr);

            let p = Pair {
                car: Some(Arc::new(car)),
                cdr,
            };

            Ok(ExprKind::Pair(Arc::new(p)))
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
                    Atom::Number(_) => Ok(ExprKind::Atom(Arc::new(Atom::Bool(
                        RLispBoolean::True(true),
                    )))),
                    _ => Ok(ExprKind::Atom(Arc::new(Atom::Bool(RLispBoolean::False(
                        false,
                    ))))),
                },
                _ => Ok(ExprKind::Atom(Arc::new(Atom::Bool(RLispBoolean::False(
                    false,
                ))))),
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
                ExprKind::List(_) => Ok(ExprKind::Atom(Arc::new(Atom::Bool(RLispBoolean::True(
                    true,
                ))))),
                _ => Ok(ExprKind::Atom(Arc::new(Atom::Bool(RLispBoolean::False(
                    false,
                ))))),
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
                    Atom::Bool(_) => Ok(ExprKind::Atom(Arc::new(Atom::Bool(RLispBoolean::True(
                        true,
                    ))))),
                    _ => Ok(ExprKind::Atom(Arc::new(Atom::Bool(RLispBoolean::False(
                        false,
                    ))))),
                },
                _ => Ok(ExprKind::Atom(Arc::new(Atom::Bool(RLispBoolean::False(
                    false,
                ))))),
            }
        }),
    );

    env.insert(
        "null?".to_string(),
        Box::new(|exp, _| {
            if exp.is_empty() {
                return Err("bool? requires one argument".to_string());
            }

            if let ExprKind::List(exp) = exp[0].clone() {
                if exp.is_empty() {
                    return Ok(ExprKind::Atom(Arc::new(Atom::Bool(RLispBoolean::True(
                        true,
                    )))));
                } else {
                    return Ok(ExprKind::Atom(Arc::new(Atom::Bool(RLispBoolean::False(
                        false,
                    )))));
                }
            } else if let ExprKind::Quote(q) = exp[0].clone() {
                if let ExprKind::List(l) = q.expr.clone() {
                    if l.is_empty() {
                        return Ok(ExprKind::Atom(Arc::new(Atom::Bool(RLispBoolean::True(
                            true,
                        )))));
                    } else {
                        return Ok(ExprKind::Atom(Arc::new(Atom::Bool(RLispBoolean::False(
                            false,
                        )))));
                    }
                } else {
                    return Err("invalid expression".to_string());
                }
            } else {
                return Err("invalid expression".to_string());
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
                    l.length() as i32,
                ))))),
                _ => Err("argument must be a list".to_string()),
            }
        }),
    );

    env.insert(
        "length".to_string(),
        Box::new(|exp, _| {
            if exp.is_empty() {
                return Err("len requires one argument".to_string());
            }

            match &exp[0] {
                ExprKind::List(l) => Ok(ExprKind::Atom(Arc::new(Atom::Number(RLispNumber::Int(
                    l.length() as i32,
                ))))),
                ExprKind::Quote(q) => {
                    match &q.expr {
                        ExprKind::List(l) => {
                            Ok(ExprKind::Atom(Arc::new(Atom::Number(RLispNumber::Int(
                                l.length() as i32,
                            )))))
                        },
                        _ => Err("argument must be a list".to_string()),
                    }
                }
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
                    if l.is_empty() {
                        Err("empty list".to_string())
                    } else {
                        Ok(l.car().unwrap().as_ref().clone())
                    }
                },
                ExprKind::Quote(q) => {
                    match q.expr.clone() {
                        ExprKind::List(l) => {
                            if l.is_empty() {
                                Err("empty list".to_string())
                            } else {
                                Ok(l.car().unwrap().as_ref().clone())
                            }
                        },

                        _ => {
                            Err("argument must be a list".to_string())
                        }
                    }
                },
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
                    if l.is_empty() {
                        Err("cdr: empty list".to_string())
                    } else {
                        Ok(ExprKind::List(Arc::new(l.cdr().unwrap_or_else(PairList::nil))))
                    }
                },
                ExprKind::Quote(q) => {
                    match q.expr.clone() {
                        ExprKind::List(l) => {
                            if l.is_empty() {
                                Err("cdr: empty list".to_string())
                            } else {
                                Ok(ExprKind::List(Arc::new(l.cdr().unwrap_or_else(PairList::nil))))
                            }
                        },
                        _ => {
                            Err("argument must be a list".to_string())
                        }
                    }
                },
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

            let l = &exp[0];
            let r = &exp[1];

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
                ExprKind::Atom(a) => match *a.to_owned() {
                    Atom::Bool(ref rlisp_boolean) => match rlisp_boolean {
                        RLispBoolean::True(_) => true,
                        RLispBoolean::False(_) => false,
                    },
                    _ => return Err("invalid expression".to_string()),
                },

                _ => return Err("invalid expression".to_string()),
            };

            let r = match &exp[1] {
                ExprKind::Atom(a) => match *a.to_owned() {
                    Atom::Bool(ref rlisp_boolean) => match rlisp_boolean {
                        RLispBoolean::True(_) => true,
                        RLispBoolean::False(_) => false,
                    },
                    _ => return Err("invalid expression".to_string()),
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
                ExprKind::Atom(a) => match *a.to_owned() {
                    Atom::Bool(ref rlisp_boolean) => match rlisp_boolean {
                        RLispBoolean::True(_) => true,
                        RLispBoolean::False(_) => false,
                    },
                    _ => return Err("invalid expression".to_string()),
                },

                _ => return Err("invalid expression".to_string()),
            };

            let r = match &exp[1] {
                ExprKind::Atom(a) => match *a.to_owned() {
                    Atom::Bool(ref rlisp_boolean) => match rlisp_boolean {
                        RLispBoolean::True(_) => true,
                        RLispBoolean::False(_) => false,
                    },
                    _ => return Err("invalid expression".to_string()),
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
                ExprKind::Atom(a) => match *a.to_owned() {
                    Atom::Bool(ref rlisp_boolean) => match rlisp_boolean {
                        RLispBoolean::True(_) => true,
                        RLispBoolean::False(_) => false,
                    },
                    _ => return Err("invalid expression".to_string()),
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
                expr: ExprKind::List(Arc::new(PairList::from_vec(exp))),
            })))
        }),
    );

    env.insert(
        "string".to_string(),
        Box::new(|exp, _| {
            let mut s = String::new();
            for c in exp.iter() {
                s = format!("{}{}", s, c);
            }

            Ok(ExprKind::StringLiteral(Arc::new(s)))
        }),
    );

    // Evaluation
    env.insert(
        "eval".to_string(),
        Box::new(|exp, symbol_defs| {
            if exp.is_empty() {
                return Err("eval requires one argument".to_string());
            }

            let list = exp[0].clone();
            if let ExprKind::Quote(q_exp) = list {
                crate::eval::eval(q_exp.expr.clone(), &std_env(), symbol_defs)
            } else {
                Err("invalid expression".to_string())
            }
        }),
    );

    env.insert(
        "quote".to_string(),
        Box::new(|exp, _| {
            Ok(ExprKind::Quote(Arc::new(Quote {
                expr: ExprKind::List(Arc::new(PairList::from_vec(exp))),
            })))
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

            // Extract list from quoted/unquoted list or valid pair
            let args = match &exp[1] {
                ExprKind::List(l) => Arc::try_unwrap(l.clone()).unwrap_or_else(|arc| (*arc).clone()),
                ExprKind::Pair(p) => {
                    if p.is_list() {
                        // Convert valid pair list to PairList
                        PairList { head: Some(p.clone()) }
                    } else {
                        return Err("Second argument to map must be a proper list".to_string());
                    }
                },
                ExprKind::Quote(args) => match args.expr.clone() {
                    ExprKind::List(l) => Arc::try_unwrap(l).unwrap_or_else(|arc| (*arc).clone()),
                    ExprKind::Pair(p) => {
                        if p.is_list() {
                            PairList { head: Some(p) }
                        } else {
                            return Err("Second argument to map must be a proper list".to_string());
                        }
                    },
                    _ => {
                        return Err("Second argument to map must be a list".to_string());
                    }
                },
                _ => {
                    return Err("Second argument to map must be a list".to_string())
                }
            };

            let mut mapping_results: Vec<ExprKind> = Vec::new();

            for arg in args.to_vec() {
                let proc =
                    ExprKind::to_proc(&ExprKind::Lambda(lambda_func.clone()), vec![arg], &env)
                        .unwrap();
                let result = proc.proc_eval(sd).unwrap();
                mapping_results.push(result);
            }

            Ok(ExprKind::Quote(Arc::new(Quote {
                expr: ExprKind::List(Arc::new(PairList::from_vec(mapping_results))),
            })))
        }),
    );

    env.insert(
        "filter".to_string(),
        Box::new(|exp, sd| {
            let env = std_env();

            let lambda_func = match &exp[0] {
                ExprKind::Lambda(lambda_exp) => lambda_exp.clone(),
                _ => return Err("First argument to map must be a lambda".to_string()),
            };

            // Extract list from quoted/unquoted list or valid pair
            let args = match &exp[1] {
                ExprKind::List(l) => Arc::try_unwrap(l.clone()).unwrap_or_else(|arc| (*arc).clone()),
                ExprKind::Pair(p) => {
                    if p.is_list() {
                        // Convert valid pair list to PairList
                        PairList { head: Some(p.clone()) }
                    } else {
                        return Err("Second argument to filter must be a proper list".to_string());
                    }
                },
                ExprKind::Quote(args) => match args.expr.clone() {
                    ExprKind::List(l) => Arc::try_unwrap(l).unwrap_or_else(|arc| (*arc).clone()),
                    ExprKind::Pair(p) => {
                        if p.is_list() {
                            PairList { head: Some(p) }
                        } else {
                            return Err("Second argument to filter must be a proper list".to_string());
                        }
                    },
                    _ => {
                        return Err("Second argument to filter must be a list".to_string());
                    }
                },
                _ => {
                    return Err("Second argument to filter must be a list".to_string())
                }
            };

            let mut mapping_results: Vec<ExprKind> = Vec::new();

            for arg in args.to_vec().iter() {
                let proc = ExprKind::to_proc(
                    &ExprKind::Lambda(lambda_func.clone()),
                    vec![arg.to_owned()],
                    &env,
                )
                .unwrap();
                let result = proc.proc_eval(sd).unwrap();
                match result {
                    ExprKind::Atom(ref atom) => {
                        if let Atom::Bool(test) = atom.as_ref() {
                            if let RLispBoolean::True(_) = test {
                                mapping_results.push(arg.clone());
                            }
                        }
                    }

                    _ => {
                        return Err("Invalid return from filter predicate".to_string());
                    }
                }
            }

            Ok(ExprKind::Quote(Arc::new(Quote {
                expr: ExprKind::List(Arc::new(PairList::from_vec(mapping_results))),
            })))
        }),
    );

    env
}
