use log::debug;

use crate::{
    op::NumericOps, types::{Atom, RLispSubSymbolicExpressions, SymbolicExpression}
};
use std::collections::HashMap;

pub fn std_const_exp() -> HashMap<String, SymbolicExpression> {
    let mut const_exps = HashMap::new();
    const_exps.insert(
        "pi".to_string(),
        SymbolicExpression::Atom("3.14".to_string()),
    );
    const_exps
}

pub fn std_env(
) -> HashMap<String, Box<dyn Fn(RLispSubSymbolicExpressions, &mut HashMap<String, SymbolicExpression>) -> Result<SymbolicExpression, String>>>
{
    let mut env: HashMap<
        String,
        Box<dyn Fn(RLispSubSymbolicExpressions, &mut HashMap<String, SymbolicExpression>) -> Result<SymbolicExpression, String>>,
    > = HashMap::new();
    env.insert(
        "+".to_string(),
        Box::new(|exp, _| -> Result<SymbolicExpression, String> {
            let l: Atom = match &exp[0] {
                SymbolicExpression::Atom(a) => a.into(),
                _ => return Err("invalid expression".to_string()),
            };

            let r: Atom = match &exp[1] {
                SymbolicExpression::Atom(a) => a.into(),
                _ => return Err("invalid expression".to_string()),

            };

            r.add(l)
        }),
    );

    env.insert(
        "-".to_string(),
        Box::new(|exp, _| -> Result<SymbolicExpression, String> {
            let l: Atom = match &exp[0] {
                SymbolicExpression::Atom(a) => a.into(),
                _ => return Err("invalid expression".to_string()),
            };

            let r: Atom = match &exp[1] {
                SymbolicExpression::Atom(a) => a.into(),
                _ => return Err("invalid expression".to_string()),
            };

            r.sub(l)
        }),
    );

    env.insert(
        "*".to_string(),
        Box::new(|exp, _| -> Result<SymbolicExpression, String> {
            let l: Atom = match &exp[0] {
                SymbolicExpression::Atom(a) => a.into(),
                _ => return Err("invalid expression".to_string()),
            };

            let r: Atom = match &exp[1] {
                SymbolicExpression::Atom(a) => a.into(),
                _ => return Err("invalid expression".to_string()),
            };

            r.mul(l)
        }),
    );

    env.insert(
        "/".to_string(),
        Box::new(|exp, _| -> Result<SymbolicExpression, String> {
            let l: Atom = match &exp[0] {
                SymbolicExpression::Atom(a) => a.into(),
                _ => return Err("invalid expression".to_string()),
            };

            let r: Atom = match &exp[1] {
                SymbolicExpression::Atom(a) => a.into(),
                _ => return Err("invalid expression".to_string()),

            };

            r.div(l)
        }),
    );

    env.insert(
        "append".to_string(),
        Box::new(|exp, _| {
            let mut l: RLispSubSymbolicExpressions =
                RLispSubSymbolicExpressions::try_from(exp[0].clone())?;
            l.push(exp[1].clone());
            Ok(SymbolicExpression::List(l))
        }),
    );
    env.insert(
        "begin".to_string(),
        Box::new(|exp, _| Ok(exp[exp.len() - 1].clone())),
    );

    env.insert(
        "display".to_string(),
        Box::new(|exp, _| {
            for i in exp.iter() {
                println!("{}", i);
            }

            Ok(SymbolicExpression::Atom("0".to_string()))
        }),
    );

    env.insert(
        "number?".to_string(),
        Box::new(|exps, _| {
            let exp = exps[0].clone();
            match exp {
                SymbolicExpression::Atom(a) => {
                    let atom: Atom = a.into();
                    match atom {
                        Atom::Number(_) => Ok(SymbolicExpression::Atom("true".to_string())),
                        Atom::Symbol(_) => Ok(SymbolicExpression::Atom("false".to_string())),
                        Atom::Bool(_) => Ok(SymbolicExpression::Atom("false".to_string())),
                    }
                }
                SymbolicExpression::List(_) => Ok(SymbolicExpression::Atom("false".to_string())),
                SymbolicExpression::Lambda(_) => Ok(SymbolicExpression::Atom("false".to_string())),
                SymbolicExpression::ListExpr(_) => Ok(SymbolicExpression::Atom("false".to_string())),
            }
        }),
    );

    env.insert(
        "list?".to_string(),
        Box::new(|exps, _| {
            let exp = exps[0].clone();
            match exp {
                SymbolicExpression::Atom(a) => {
                    let atom: Atom = a.into();
                    match atom {
                        _ => Ok(SymbolicExpression::Atom("false".to_string())),

                    }
                }
                SymbolicExpression::List(_) => Ok(SymbolicExpression::Atom("true".to_string())),
                SymbolicExpression::ListExpr(_) => Ok(SymbolicExpression::Atom("true".to_string())),
                _ => Ok(SymbolicExpression::Atom("true".to_string())),
            }
        }),
    );

    env.insert(
        "bool?".to_string(),
        Box::new(|exps, _| {
            let exp = exps[0].clone();
            match exp {
                SymbolicExpression::Atom(a) => {
                    let atom: Atom = a.into();
                    match atom {
                        Atom::Number(_) => Ok(SymbolicExpression::Atom("false".to_string())),
                        Atom::Symbol(_) => Ok(SymbolicExpression::Atom("false".to_string())),
                        Atom::Bool(_) => Ok(SymbolicExpression::Atom("true".to_string())),
                    }
                }
                _ => Ok(SymbolicExpression::Atom("false".to_string())),
            }
        }),
    );

    env.insert(
        "len".to_string(),
        Box::new(|exps, _| {
            let exp = exps[0].clone();
            match exp {
                SymbolicExpression::List(l) => Ok(SymbolicExpression::Atom(l.len().to_string())),
                SymbolicExpression::ListExpr(l) => Ok(SymbolicExpression::Atom(l.len().to_string())),
                _ => Err("invalid expression".to_string()),
            }
        }),
    );

    env.insert(
        "car".to_string(),
        Box::new(|exps, _| {
            let exp = exps[0].clone();
            match exp {
                SymbolicExpression::Atom(_) => {
                    Err("invalid expression".to_string())
                },
                SymbolicExpression::List(l) => {
                    debug!("first element {}", l[0]);
                    Ok(l[0].clone())
                },
                SymbolicExpression::Lambda(_) => Err("invalid expression".to_string()),
                SymbolicExpression::ListExpr(l) => {
                    debug!("first element {}", l[0]);
                    Ok(l[0].clone())
                },
            }
        }),
    );

    env.insert(
        "cdr".to_string(),
        Box::new(|exps, _| {
            let exp = exps[0].clone();
            match exp {
                SymbolicExpression::Atom(_) => Err("invalid expression".to_string()),
                SymbolicExpression::List(l) => Ok(SymbolicExpression::List(l[1..l.len()].to_vec())),
                SymbolicExpression::Lambda(_) => Err("invalid expression".to_string()),
                SymbolicExpression::ListExpr(l) => Ok(SymbolicExpression::List(l[1..l.len()].to_vec())),
            }
        }),
    );

    env.insert(
        "<".to_string(),
        Box::new(|exp, _| -> Result<SymbolicExpression, String> {
            let l: Atom = match &exp[0] {
                SymbolicExpression::Atom(a) => a.into(),
                _ => return Err("invalid expression".to_string()),
            };

            let r: Atom = match &exp[1] {
                SymbolicExpression::Atom(a) => a.into(),
                _ => return Err("invalid expression".to_string()),
            };

            let res = l < r;

            Ok(SymbolicExpression::Atom(res.to_string()))
        }),
    );

    env.insert(
        ">".to_string(),
        Box::new(|exp, _| -> Result<SymbolicExpression, String> {
            let l: Atom = match &exp[0] {
                SymbolicExpression::Atom(a) => a.into(),
                _ => return Err("invalid expression".to_string()),
            };

            let r: Atom = match &exp[1] {
                SymbolicExpression::Atom(a) => a.into(),
                _ => return Err("invalid expression".to_string()),
            };

            let res = l > r;

            Ok(SymbolicExpression::Atom(res.to_string()))
        }),
    );

    env.insert(
        "map".to_string(),
        Box::new(|exp, sd| -> Result<SymbolicExpression, String> {
            let env = std_env();

            let lambda_func = match &exp[0] {
                SymbolicExpression::Lambda(lambda_exp) => lambda_exp.clone(),
                _ => return Err("First argument to map must be a lambda".to_string()),
            };

            let args = match &exp[1] {
                SymbolicExpression::List(args) => args,
                _ => return Err("Second argument to map must be a list".to_string()),
            };

            let mut mapping_results: Vec<SymbolicExpression> = Vec::new();

            for arg in args {
                let lambda_call = vec![
                    SymbolicExpression::Lambda(lambda_func.clone()),
                    arg.clone()
                ];
                debug!("lambda {:?} arguments {}",lambda_func, arg.to_string());
                let result = crate::eval::eval(
                    &SymbolicExpression::Lambda(lambda_call),
                    &env,
                    sd,
                )?;

                mapping_results.push(result);
            }

            Ok(SymbolicExpression::List(mapping_results))
        }),
    );

    env.insert(
        "eval".to_string(),
        Box::new(|exp, sd| -> Result<SymbolicExpression, String> {
            let env = std_env();
            crate::eval::eval(&SymbolicExpression::List(exp.clone()), &env, sd)

        }),
    );

    env.insert(
        "list".to_string(),
        Box::new(|exp, _| -> Result<SymbolicExpression, String> {
            Ok(SymbolicExpression::List(exp))
        }),
    );

    env
}
