use log::debug;

use crate::{
    op::NumericOps,
    proc::Proc,
    types::{Atom, RLispSubSymbolicExpressions, SymbolicExpression},
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
) -> HashMap<String, Box<dyn Fn(RLispSubSymbolicExpressions) -> Result<SymbolicExpression, String>>>
{
    let mut env: HashMap<
        String,
        Box<dyn Fn(RLispSubSymbolicExpressions) -> Result<SymbolicExpression, String>>,
    > = HashMap::new();
    env.insert(
        "+".to_string(),
        Box::new(|exp| -> Result<SymbolicExpression, String> {
            let l: Atom = match &exp[0] {
                SymbolicExpression::Atom(a) => a.into(),
                SymbolicExpression::List(_) => return Err("invalid expression".to_string()),
                SymbolicExpression::Lambda(_) => return Err("invalid expression".to_string()),
            };

            let r: Atom = match &exp[1] {
                SymbolicExpression::Atom(a) => a.into(),
                SymbolicExpression::List(_) => return Err("invalid expression".to_string()),
                SymbolicExpression::Lambda(_) => return Err("invalid expression".to_string()),
            };

            r.add(l)
        }),
    );

    env.insert(
        "-".to_string(),
        Box::new(|exp| -> Result<SymbolicExpression, String> {
            let l: Atom = match &exp[0] {
                SymbolicExpression::Atom(a) => a.into(),
                SymbolicExpression::List(_) => return Err("invalid expression".to_string()),
                SymbolicExpression::Lambda(_) => return Err("invalid expression".to_string()),
            };

            let r: Atom = match &exp[1] {
                SymbolicExpression::Atom(a) => a.into(),
                SymbolicExpression::List(_) => return Err("invalid expression".to_string()),
                SymbolicExpression::Lambda(_) => return Err("invalid expression".to_string()),
            };

            r.sub(l)
        }),
    );

    env.insert(
        "*".to_string(),
        Box::new(|exp| -> Result<SymbolicExpression, String> {
            let l: Atom = match &exp[0] {
                SymbolicExpression::Atom(a) => a.into(),
                SymbolicExpression::List(_) => return Err("invalid expression".to_string()),
                SymbolicExpression::Lambda(_) => return Err("invalid expression".to_string()),
            };

            let r: Atom = match &exp[1] {
                SymbolicExpression::Atom(a) => a.into(),
                SymbolicExpression::List(_) => return Err("invalid expression".to_string()),
                SymbolicExpression::Lambda(_) => return Err("invalid expression".to_string()),
            };

            r.mul(l)
        }),
    );

    env.insert(
        "/".to_string(),
        Box::new(|exp| -> Result<SymbolicExpression, String> {
            let l: Atom = match &exp[0] {
                SymbolicExpression::Atom(a) => a.into(),
                SymbolicExpression::List(_) => return Err("invalid expression".to_string()),
                SymbolicExpression::Lambda(_) => return Err("invalid expression".to_string()),
            };

            let r: Atom = match &exp[1] {
                SymbolicExpression::Atom(a) => a.into(),
                SymbolicExpression::List(_) => return Err("invalid expression".to_string()),
                SymbolicExpression::Lambda(_) => return Err("invalid expression".to_string()),
            };

            r.div(l)
        }),
    );

    env.insert(
        "append".to_string(),
        Box::new(|exp| {
            let mut l: RLispSubSymbolicExpressions =
                RLispSubSymbolicExpressions::try_from(exp[0].clone())?;
            l.push(exp[1].clone());
            Ok(SymbolicExpression::List(l))
        }),
    );
    env.insert(
        "begin".to_string(),
        Box::new(|exp| Ok(exp[exp.len() - 1].clone())),
    );

    env.insert(
        "print".to_string(),
        Box::new(|exp| {
            for i in exp.iter() {
                println!("{}", i);
            }

            Ok(SymbolicExpression::Atom("0".to_string()))
        }),
    );

    env.insert(
        "number?".to_string(),
        Box::new(|exps| {
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
            }
        }),
    );

    env.insert(
        "list?".to_string(),
        Box::new(|exps| {
            let exp = exps[0].clone();
            match exp {
                SymbolicExpression::Atom(a) => {
                    let atom: Atom = a.into();
                    match atom {
                        Atom::Number(_) => Ok(SymbolicExpression::Atom("false".to_string())),
                        Atom::Symbol(_) => Ok(SymbolicExpression::Atom("false".to_string())),
                        Atom::Bool(_) => Ok(SymbolicExpression::Atom("false".to_string())),
                    }
                }
                SymbolicExpression::List(_) => Ok(SymbolicExpression::Atom("true".to_string())),
                SymbolicExpression::Lambda(_) => Ok(SymbolicExpression::Atom("false".to_string())),
            }
        }),
    );

    env.insert(
        "bool?".to_string(),
        Box::new(|exps| {
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
                SymbolicExpression::List(_) => Ok(SymbolicExpression::Atom("false".to_string())),
                SymbolicExpression::Lambda(_) => Ok(SymbolicExpression::Atom("false".to_string())),
            }
        }),
    );

    env.insert(
        "len".to_string(),
        Box::new(|exps| {
            let exp = exps[0].clone();
            match exp {
                SymbolicExpression::Atom(_) => Err("invalid expression".to_string()),
                SymbolicExpression::List(l) => Ok(SymbolicExpression::Atom(l.len().to_string())),
                SymbolicExpression::Lambda(_) => Err("invalid expression".to_string()),
            }
        }),
    );

    env.insert(
        "car".to_string(),
        Box::new(|exps| {
            let exp = exps[0].clone();
            match exp {
                SymbolicExpression::Atom(_) => Err("invalid expression".to_string()),
                SymbolicExpression::List(l) => Ok(l[0].clone()),
                SymbolicExpression::Lambda(_) => Err("invalid expression".to_string()),
            }
        }),
    );

    env.insert(
        "cdr".to_string(),
        Box::new(|exps| {
            let exp = exps[0].clone();
            match exp {
                SymbolicExpression::Atom(_) => Err("invalid expression".to_string()),
                SymbolicExpression::List(l) => Ok(SymbolicExpression::List(l[1..l.len()].to_vec())),
                SymbolicExpression::Lambda(_) => Err("invalid expression".to_string()),
            }
        }),
    );

    env.insert(
        "<".to_string(),
        Box::new(|exp| -> Result<SymbolicExpression, String> {
            let l: Atom = match &exp[0] {
                SymbolicExpression::Atom(a) => a.into(),
                SymbolicExpression::List(_) => return Err("invalid expression".to_string()),
                SymbolicExpression::Lambda(_) => return Err("invalid expression".to_string()),
            };

            let r: Atom = match &exp[1] {
                SymbolicExpression::Atom(a) => a.into(),
                SymbolicExpression::List(_) => return Err("invalid expression".to_string()),
                SymbolicExpression::Lambda(_) => return Err("invalid expression".to_string()),
            };

            let res = l < r;

            Ok(SymbolicExpression::Atom(res.to_string()))
        }),
    );

    env.insert(
        ">".to_string(),
        Box::new(|exp| -> Result<SymbolicExpression, String> {
            let l: Atom = match &exp[0] {
                SymbolicExpression::Atom(a) => a.into(),
                SymbolicExpression::List(_) => return Err("invalid expression".to_string()),
                SymbolicExpression::Lambda(_) => return Err("invalid expression".to_string()),
            };

            let r: Atom = match &exp[1] {
                SymbolicExpression::Atom(a) => a.into(),
                SymbolicExpression::List(_) => return Err("invalid expression".to_string()),
                SymbolicExpression::Lambda(_) => return Err("invalid expression".to_string()),
            };

            let res = l > r;

            Ok(SymbolicExpression::Atom(res.to_string()))
        }),
    );

    env.insert(
        "map".to_string(),
        Box::new(|exp| -> Result<SymbolicExpression, String> {
            let env = std_env();
            let l: Proc = match &exp[0] {
                SymbolicExpression::Atom(_) => return Err("invalid expression".to_string()),
                SymbolicExpression::Lambda(exp) => {
                    match SymbolicExpression::to_proc(exp.to_owned(), &env) {
                        Ok(proc) => proc,
                        Err(_) => return Err("Invalid expresssion expected procedure".to_string()),
                    }
                }
                SymbolicExpression::List(_) => return Err("Invalid expression".to_string()),
            };
            debug!("resolved proc: {}, from definition", l);
            match &exp[1] {
                SymbolicExpression::Atom(_) => return Err("invalid expression".to_string()),
                SymbolicExpression::List(args) => {
                    let mut mapping_results: Vec<SymbolicExpression> = Vec::new();
                    for a in args {
                        log::debug!("running mapping on {}", a);
                        let proc_with_args: SymbolicExpression =
                            SymbolicExpression::List([l.body.clone(), a.to_owned()].to_vec());
                        let mapped =
                            crate::eval::map_args(proc_with_args, l.signature.clone(), &env)
                                .unwrap();
                        debug!("mapped args {:?}", mapped);
                        match mapped {
                            SymbolicExpression::Atom(_) => todo!(),
                            SymbolicExpression::List(l) => {
                                let res = crate::eval::eval(
                                    &l[0].clone(),
                                    &std_env(),
                                    &mut HashMap::new(),
                                );

                                debug!("res from applying argument to function: {:?}", res);
                                mapping_results.push(res.unwrap());
                            }
                            SymbolicExpression::Lambda(_) => todo!(),
                        }
                    }

                    return Ok(SymbolicExpression::List(mapping_results));
                }
                SymbolicExpression::Lambda(_) => return Err("Invalid expression".to_string()),
            }
        }),
    );

    env
}
