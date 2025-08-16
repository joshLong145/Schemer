use log::debug;

use crate::proc::{Eval, Proc};
use crate::types::{Atom, RLispSubSymbolicExpressions, SymbolicExpression};
use std::collections::HashMap;

pub fn eval(
    expression: &SymbolicExpression,
    env: &HashMap<
        String,
        Box<dyn Fn(RLispSubSymbolicExpressions, &mut HashMap<String, SymbolicExpression>) -> Result<SymbolicExpression, String>>,
    >,
    symbol_definitions: &mut HashMap<String, SymbolicExpression>,
) -> Result<SymbolicExpression, String> {
    debug!("starting expression eval");
    match expression {
        SymbolicExpression::Atom(symbolic_atom) => {
            debug!("found expression to be of type atom {}", symbolic_atom);
            let atom: Atom = symbolic_atom.into();
            match atom {
                Atom::Number(_) => {
                    return Ok(expression.clone());
                }
                Atom::Symbol(s) => {
                    debug!("looking up symbol definition for {}", s);
                    if let Some(symbol) = symbol_definitions.get(&s) {
                        debug!("found symbol definition {}", symbol);
                        Ok(symbol.clone())
                    } else {
                        Err(format!("invalid symbol {}", symbolic_atom))
                    }
                }
                Atom::Bool(_) => Ok(expression.clone()),
            }
        }
        SymbolicExpression::List(vec) => {
            debug!("found expression to be of type list, resolving first expression of list");
            let expression = vec[0].clone();
            match expression {
                SymbolicExpression::Atom(exp) => {
                    // implement resolving of symbols for atoms before attempting to evaulate further epxressions
                    let atom: Atom = exp.into();
                    match atom {
                        Atom::Number(_) => Ok(SymbolicExpression::List(vec.clone())),
                        Atom::Bool(_) => Ok(SymbolicExpression::List(vec.clone())),
                        Atom::Symbol(exp) => {
                            debug!("found first expression to be symbol");
                            if exp == "define" {
                                debug!("found sybmol to be definition stmt, processing rest of list");
                                let symbol = vec[1].clone();
                                let e = vec[2].clone();
                                debug!("definition symbol {}, expression {}", symbol, e);
                                debug!("resolving expression for define");
                                match e.clone() {
                                    SymbolicExpression::Atom(_) => {
                                        debug!("found expression to be atom, elavuating");
                                        let val = eval(&e, env, symbol_definitions).unwrap();
                                        symbol_definitions.insert(symbol.to_string(), val.clone());
                                        debug!("evaluated expression, result {}", val);
                                        return Ok(symbol);
                                    }
                                    SymbolicExpression::List(sub_exp) => {
                                        debug!("found expression to be list, evaluating");
                                        let p = sub_exp[0].clone();
                                        match p {
                                            SymbolicExpression::Atom(sym_exp) => {
                                                debug!("looking up symbol definitions for sub expression {}", sym_exp);
                                                if let Some(_) = symbol_definitions.get(&sym_exp) {
                                                    return Err(format!(
                                                        "{} is already defined",
                                                        sym_exp
                                                    ));
                                                } else {
                                                    debug!("defining symbol {} sub exp {:?}", symbol, sub_exp);
                                                    symbol_definitions.insert(
                                                        symbol.to_string(),
                                                        SymbolicExpression::List(sub_exp),
                                                    );
                                                    return Ok(symbol.clone());
                                                }
                                            }
                                            SymbolicExpression::List(e) => {
                                                return Err(format!("Invalid expression {:?}", e));
                                            }
                                            SymbolicExpression::Lambda(_) => todo!(),
                                        }
                                    }
                                    SymbolicExpression::Lambda(_) => todo!(),
                                };
                            }
                            if exp == "lambda" {
                                let sym_exp = SymbolicExpression::Lambda(vec.clone());
                                eval(&sym_exp, env, symbol_definitions)
                            } else if exp == "if" {
                                let test =
                                    resolve_symbol_if_present(&vec[1], symbol_definitions, env);

                                let test_res = eval(&test, env, symbol_definitions).unwrap();
                                match test_res {
                                    SymbolicExpression::Atom(ts) => {
                                        let ts: Atom = ts.into();
                                        match ts {
                                            Atom::Number(_) => {
                                                Err("invalid expression".to_string())
                                            }
                                            Atom::Symbol(_) => {
                                                Err("invalid expression".to_string())
                                            }
                                            Atom::Bool(boolean) => match boolean {
                                                crate::types::RLispBoolean::True(_) => {
                                                    if SymbolicExpression::is_proc(&vec[2]) {
                                                        return eval(
                                                            &SymbolicExpression::Lambda(
                                                                vec[2].clone().try_into().unwrap(),
                                                            ),
                                                            env,
                                                            symbol_definitions,
                                                        );
                                                    }
                                                    eval(&vec[2], env, symbol_definitions)
                                                }
                                                crate::types::RLispBoolean::False(_) => {
                                                    if SymbolicExpression::is_proc(&vec[3]) {
                                                        return eval(
                                                            &SymbolicExpression::Lambda(
                                                                vec[3].clone().try_into().unwrap(),
                                                            ),
                                                            env,
                                                            symbol_definitions,
                                                        );
                                                    }
                                                    eval(&vec[3], env, symbol_definitions)
                                                }
                                            },
                                        }
                                    }
                                    SymbolicExpression::List(_) => {
                                        Err("invalid expression".to_string())
                                    }
                                    SymbolicExpression::Lambda(_) => todo!(),
                                }
                            } else {
                                // procedure call
                                if let Some(proc) = env.get(&exp) {
                                    let args: Result<Vec<SymbolicExpression>, String> = vec[1..vec.len()]
                                        .iter()
                                        .map(|se| {
                                            let e = resolve_symbol_if_present(
                                                se,
                                                symbol_definitions,
                                                env,
                                            );
                                            if SymbolicExpression::is_proc(&e) {
                                                return eval(
                                                    &SymbolicExpression::Lambda(
                                                        e.try_into().unwrap(),
                                                    ),
                                                    env,
                                                    symbol_definitions,
                                                );
                                            }
                                            eval(&e, env, symbol_definitions)
                                        })
                                        .collect();
                                    let args = args?;
                                    proc(args, symbol_definitions)
                                } else {
                                    let func = symbol_definitions.get(&exp);

                                    match func.cloned() {
                                        Some(f) => {
                                            match f {
                                                SymbolicExpression::Atom(e) => {
                                                    return Err(format!(
                                                        "Invalid expression for proc {}",
                                                        e
                                                    ));
                                                }
                                                SymbolicExpression::List(sub_exp) => {
                                                    let args: Vec<SymbolicExpression> = vec[1..].to_vec();
                                                    let mut lambda_with_args = sub_exp.clone();
                                                    lambda_with_args.extend(args);
                                                    debug!(
                                                        "invoke proc looked up from symbol: {:?}",
                                                        lambda_with_args
                                                    );

                                                    return eval(
                                                        &SymbolicExpression::Lambda(lambda_with_args),
                                                        env,
                                                        symbol_definitions,
                                                    );
                                                }
                                                SymbolicExpression::Lambda(_) => todo!(),
                                            }
                                        }

                                        None => {
                                            return Err(format!("{} is not defined", exp));
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
                SymbolicExpression::List(vec) => Ok(SymbolicExpression::List(vec)),
                SymbolicExpression::Lambda(l) => Ok(SymbolicExpression::Lambda(l)),
            }
        }
        SymbolicExpression::Lambda(exp) => {
            if exp.len() == 3 {
                return Ok(SymbolicExpression::Lambda(exp.clone().try_into().unwrap()));
            } else {
                let lambda: Vec<SymbolicExpression> = exp[0].clone().try_into().unwrap();

                let mut params = HashMap::new();
                let lambda_params = &lambda[1];
                let call_args = &exp[1..]; // Arguments passed to the lambda

                match lambda_params {
                    SymbolicExpression::List(param_list) => {
                        for (i, param) in param_list.iter().enumerate() {
                            if i < call_args.len() {
                                let arg_value = eval(&call_args[i], env, symbol_definitions)?;
                                params.insert(param.to_string(), arg_value.clone());
                            }
                        }
                    }
                    SymbolicExpression::Atom(param_name) => {
                        if !call_args.is_empty() {
                            let arg_value = eval(&call_args[0], env, symbol_definitions)?;
                            params.insert(param_name.to_string(), arg_value.clone());
                        }
                    }
                    _ => return Err("Invalid lambda parameter specification".to_string()),
                }

                let proc = Proc {
                    body: lambda[2].clone(), // Lambda body is the third element
                    params,
                    env,
                    signature: lambda_params.clone(),
                };

                debug!("calling proc {}", proc);
                proc.proc_eval(symbol_definitions)
            }
        }
    }
}

pub fn map_args(
    exp: SymbolicExpression,
    arguements: SymbolicExpression,
    env: &HashMap<
        String,
        Box<dyn Fn(RLispSubSymbolicExpressions, &mut HashMap<String, SymbolicExpression>) -> Result<SymbolicExpression, String>>,
    >,
) -> Result<SymbolicExpression, String> {
    let param_pairs: &mut HashMap<String, SymbolicExpression> = &mut HashMap::new();
    match exp.clone() {
        SymbolicExpression::Atom(_) => {
            return Err("invalid expression".to_string());
        }
        SymbolicExpression::List(sub_exp) => {
            let params = sub_exp[1].clone();
            match params {
                SymbolicExpression::Atom(p) => {
                    let arg = match arguements {
                        SymbolicExpression::Atom(ref a) => {
                            SymbolicExpression::Atom(a.clone())
                        }
                        SymbolicExpression::List(ref vec) => {
                            vec[0].clone()
                        }
                        SymbolicExpression::Lambda(ref l) => SymbolicExpression::Lambda(l.clone()),
                    };

                    param_pairs.insert(p.to_string(), arg);
                }
                SymbolicExpression::List(p) => {
                    for i in 0..p.len() {
                        let arg = match arguements {
                            SymbolicExpression::Atom(ref a) => {
                                if i == 0 {
                                    SymbolicExpression::Atom(a.clone())
                                } else {
                                    return Err("Too many parameters for single argument".to_string());
                                }
                            }
                            SymbolicExpression::List(ref vec) => {
                                if i < vec.len() {
                                    vec[i].clone()
                                } else {
                                    return Err("Not enough arguments provided".to_string());
                                }
                            }
                            SymbolicExpression::Lambda(ref l) => {
                                SymbolicExpression::Lambda(l.clone())
                            }
                        };

                        let param = p[i].clone();
                        param_pairs.insert(param.to_string(), arg);
                    }
                }
                SymbolicExpression::Lambda(p) => {
                    for i in 0..p.len() {
                        let arg = match arguements {
                            SymbolicExpression::Atom(ref a) => {
                                if i == 0 {
                                    SymbolicExpression::Atom(a.clone())
                                } else {
                                    return Err("Too many parameters for single argument".to_string());
                                }
                            }
                            SymbolicExpression::List(ref vec) => {
                                if i < vec.len() {
                                    vec[i].clone()
                                } else {
                                    return Err("Not enough arguments provided".to_string());
                                }
                            }
                            SymbolicExpression::Lambda(ref l) => {
                                SymbolicExpression::Lambda(l.clone())
                            }
                        };

                        let param = p[i].clone();
                        param_pairs.insert(param.to_string(), arg);
                    }
                }
            }
        }
        SymbolicExpression::Lambda(_) => todo!(),
    };
    let res = resolve_symbol_if_present(&exp, param_pairs, env);
    Ok(res)
}

fn resolve_symbol_if_present(
    se: &SymbolicExpression,
    symbol_definitions: &mut HashMap<String, SymbolicExpression>,
    env: &HashMap<
        String,
        Box<dyn Fn(RLispSubSymbolicExpressions, &mut HashMap<String, SymbolicExpression>) -> Result<SymbolicExpression, String>>,
    >,
) -> SymbolicExpression {
    match se {
        SymbolicExpression::Atom(a) => {
            if let Some(e) = symbol_definitions.get(a) {
                e.clone()
            } else {
                se.clone()
            }
        }
        SymbolicExpression::List(l) => {
            let mut sub_exps: Vec<SymbolicExpression> = vec![];
            for e in l {
                let res = resolve_symbol_if_present(e, symbol_definitions, env);
                sub_exps.push(res);
            }

            SymbolicExpression::List(sub_exps)
        }
        SymbolicExpression::Lambda(l) => {
            let mut sub_exps: Vec<SymbolicExpression> = vec![];
            for e in l {
                let res = resolve_symbol_if_present(e, symbol_definitions, env);
                sub_exps.push(res);
            }

            SymbolicExpression::Lambda(sub_exps)
        }
    }
}
