use log::debug;

use crate::proc::{Eval, Proc};
use crate::types::{Atom, RLispSubSymbolicExpressions, SymbolicExpression};
use std::collections::HashMap;

pub fn eval(
    expression: &SymbolicExpression,
    env: &HashMap<
        String,
        Box<dyn Fn(RLispSubSymbolicExpressions) -> Result<SymbolicExpression, String>>,
    >,
    symbol_definitions: &mut HashMap<String, SymbolicExpression>,
) -> Result<SymbolicExpression, String> {
    match expression {
        SymbolicExpression::Atom(symbolic_atom) => {
            let atom: Atom = symbolic_atom.into();
            match atom {
                Atom::Number(_) => {
                    return Ok(expression.clone());
                }
                Atom::Symbol(s) => {
                    if let Some(symbol) = symbol_definitions.get(&s) {
                        Ok(symbol.clone())
                    } else {
                        Err(format!("invalid symbol {}", symbolic_atom))
                    }
                }
                Atom::Bool(_) => Ok(expression.clone()),
            }
        }
        SymbolicExpression::List(vec) => {
            let expression = vec[0].clone();
            match expression {
                SymbolicExpression::Atom(exp) => {
                    // implement resolving of symbols for atoms before attempting to evaulate further epxressions
                    let atom: Atom = exp.into();
                    match atom {
                        // if
                        Atom::Number(_) => Ok(SymbolicExpression::List(vec.clone())),
                        Atom::Bool(_) => Ok(SymbolicExpression::List(vec.clone())),
                        Atom::Symbol(exp) => {
                            if exp == "define" {
                                let symbol = vec[1].clone();
                                let e = vec[2].clone();
                                debug!("expression for define {}", e);

                                match e.clone() {
                                    SymbolicExpression::Atom(_) => {
                                        let val = eval(&e, env, symbol_definitions).unwrap();
                                        symbol_definitions.insert(symbol.to_string(), val);

                                        return Ok(symbol);
                                    }
                                    SymbolicExpression::List(sub_exp) => {
                                        let p = sub_exp[0].clone();
                                        match p {
                                            SymbolicExpression::Atom(sym_exp) => {
                                                if let Some(e) = symbol_definitions.get(&sym_exp) {
                                                    return Err(format!(
                                                        "Invalid expression {}",
                                                        e
                                                    ));
                                                } else {
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
                                        }
                                    }
                                };
                            }
                            if exp == "lambda" {
                                let local_symbol_map: HashMap<String, SymbolicExpression> =
                                    HashMap::new();

                                let body = vec[2].clone();

                                let proc = Proc {
                                    params: local_symbol_map,
                                    body: body.clone(),
                                    env,
                                };
                                debug!("body for lambda {}", body);
                                proc.proc_eval(symbol_definitions)
                            } else if exp == "if" {
                                let test =
                                    resolve_symbol_if_present(&vec[1], symbol_definitions, env);
                                let consq =
                                    resolve_symbol_if_present(&vec[2], symbol_definitions, env);
                                let cond_exp =
                                    resolve_symbol_if_present(&vec[3], symbol_definitions, env);

                                let test_res = eval(&test, env, symbol_definitions).unwrap();
                                debug!("evaluation of test for condition {} {}", test, test_res);
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
                                                    eval(&consq, env, symbol_definitions)
                                                }
                                                crate::types::RLispBoolean::False(_) => {
                                                    eval(&cond_exp, env, symbol_definitions)
                                                }
                                            },
                                        }
                                    }
                                    SymbolicExpression::List(_) => {
                                        Err("invalid expression".to_string())
                                    }
                                }
                            } else {
                                // procedure call
                                if let Some(proc) = env.get(&exp) {
                                    let args = vec[1..vec.len()]
                                        .iter()
                                        .map(|se| {
                                            let e = resolve_symbol_if_present(
                                                se,
                                                symbol_definitions,
                                                env,
                                            );
                                            debug!("resolved symbol args for {} {}", exp, e);

                                            if let Some(p) = e.try_peek() {
                                                if let Some(q) = p.try_peek() {
                                                    if q.to_string() == "lambda" {
                                                        let lambda_invoke = e.clone();
                                                        match lambda_invoke {
                                                            SymbolicExpression::Atom(_) => todo!(),
                                                            SymbolicExpression::List(vec) => {
                                                                debug!(
                                                                    "found lambda to invoke {:?}",
                                                                    vec
                                                                );
                                                                let params = vec[1].clone();
                                                                let mapped_exp = map_args(
                                                                    vec[0].clone(),
                                                                    params,
                                                                    env,
                                                                )
                                                                .unwrap();

                                                                return eval(
                                                                    &mapped_exp,
                                                                    env,
                                                                    symbol_definitions,
                                                                )
                                                                .unwrap();
                                                            }
                                                        }
                                                    }
                                                }
                                            }

                                            let res = eval(&e, env, symbol_definitions).unwrap();
                                            debug!("building args, current: {}", res);
                                            res
                                        })
                                        .collect();
                                    debug!("proc {} args {:?}", exp, args);
                                    proc(args)
                                } else {
                                    let sd = symbol_definitions.clone();
                                    let func = sd.get(&exp);

                                    match func.clone() {
                                        Some(f) => {
                                            debug!(
                                                "looked up lambda from stored exp: {:?} from label: {}",
                                                f, exp
                                            );

                                            match f {
                                                SymbolicExpression::Atom(e) => {
                                                    return Err(format!(
                                                        "Invalid expression {}",
                                                        e
                                                    ));
                                                }
                                                SymbolicExpression::List(sub_exp) => {
                                                    let params = vec[1].clone();
                                                    debug!("lambda parameters {}", params);
                                                    let mapped_exp = map_args(
                                                        SymbolicExpression::List(sub_exp.clone()),
                                                        params,
                                                        env,
                                                    )
                                                    .unwrap();
                                                    debug!("mapped expression: {}", mapped_exp);
                                                    return Ok(eval(
                                                        &mapped_exp,
                                                        env,
                                                        symbol_definitions,
                                                    )
                                                    .unwrap());
                                                }
                                            }
                                        }

                                        None => {
                                            return Err(format!("Invalid expression {}", exp));
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
                SymbolicExpression::List(vec) => Ok(SymbolicExpression::List(vec)),
            }
        }
    }
}

fn map_args(
    exp: SymbolicExpression,
    arguements: SymbolicExpression,
    env: &HashMap<
        String,
        Box<dyn Fn(RLispSubSymbolicExpressions) -> Result<SymbolicExpression, String>>,
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
                SymbolicExpression::Atom(_) => {
                    return Ok(exp.clone());
                }
                SymbolicExpression::List(p) => {
                    for i in 0..p.len() {
                        let exp = match arguements {
                            SymbolicExpression::Atom(_) => {
                                return Ok(exp.clone());
                            }
                            SymbolicExpression::List(ref vec) => {
                                debug!("found argument to lambda: {} arg: {}", exp, vec[i]);
                                vec[i].clone()
                            }
                        };

                        let symbol = p[i].clone();
                        param_pairs.insert(symbol.to_string(), exp);
                    }
                }
            }
        }
    };
    let res = resolve_symbol_if_present(&exp, param_pairs, env);
    debug!(
        "final expression after param mapping {} parameter map: {:?}",
        res, param_pairs
    );
    Ok(res)
}

fn resolve_symbol_if_present(
    se: &SymbolicExpression,
    symbol_definitions: &mut HashMap<String, SymbolicExpression>,
    env: &HashMap<
        String,
        Box<dyn Fn(RLispSubSymbolicExpressions) -> Result<SymbolicExpression, String>>,
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
    }
}
