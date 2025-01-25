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
                                                if let Some(_) = symbol_definitions.get(&sym_exp) {
                                                    return Err(format!(
                                                        "{} is already defined",
                                                        sym_exp
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
                                            SymbolicExpression::Lambda(_) => todo!(),
                                        }
                                    }
                                    SymbolicExpression::Lambda(_) => todo!(),
                                };
                            }
                            if exp == "lambda" {
                                let sym_exp = SymbolicExpression::Lambda(vec.clone());
                                debug!("Found lambda {:?}", vec);
                                eval(&sym_exp, env, symbol_definitions)
                            } else if exp == "if" {
                                let test =
                                    resolve_symbol_if_present(&vec[1], symbol_definitions, env);

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
                                                    eval(&vec[2], env, symbol_definitions)
                                                }
                                                crate::types::RLispBoolean::False(_) => {
                                                    debug!(
                                                        "cond expression for eval: {:?}",
                                                        vec[3]
                                                    );
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
                                    let args = vec[1..vec.len()]
                                        .iter()
                                        .map(|se| {
                                            let e = resolve_symbol_if_present(
                                                se,
                                                symbol_definitions,
                                                env,
                                            );
                                            if SymbolicExpression::is_proc(&e) {
                                                debug!("resolved symbol args for {} {}", exp, e);
                                                return eval(
                                                    &SymbolicExpression::Lambda(
                                                        e.try_into().unwrap(),
                                                    ),
                                                    env,
                                                    symbol_definitions,
                                                )
                                                .unwrap();
                                            }
                                            debug!("resolved symbol args for {} {}", exp, e);
                                            eval(&e, env, symbol_definitions).unwrap()
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
                                                        "Invalid expression for proc {}",
                                                        e
                                                    ));
                                                }
                                                SymbolicExpression::List(sub_exp) => {
                                                    let l_args: Vec<SymbolicExpression> = vec
                                                        [vec.len() - 1]
                                                        .clone()
                                                        .try_into()
                                                        .unwrap();
                                                    let l_invoke: Vec<SymbolicExpression> = vec![
                                                        SymbolicExpression::List(sub_exp.clone()),
                                                        SymbolicExpression::List(l_args),
                                                    ]
                                                    .iter()
                                                    .map(|se| {
                                                        let e = resolve_symbol_if_present(
                                                            se,
                                                            symbol_definitions,
                                                            env,
                                                        );
                                                        if SymbolicExpression::is_proc(&e) {
                                                            debug!(
                                                                "resolved symbol args for {} {}",
                                                                exp, e
                                                            );
                                                            return eval(
                                                                &SymbolicExpression::Lambda(
                                                                    e.try_into().unwrap(),
                                                                ),
                                                                env,
                                                                symbol_definitions,
                                                            )
                                                            .unwrap();
                                                        }
                                                        debug!(
                                                            "resolved symbol args for {} {}",
                                                            exp, e
                                                        );
                                                        eval(&e, env, symbol_definitions).unwrap()
                                                    })
                                                    .collect();
                                                    debug!(
                                                        "invoke proc looked up from symbol: {:?}",
                                                        l_invoke
                                                    );
                                                    return Ok(eval(
                                                        &SymbolicExpression::Lambda(
                                                            l_invoke.to_owned(),
                                                        ),
                                                        env,
                                                        symbol_definitions,
                                                    )
                                                    .unwrap());
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
                debug!("found lambda definition returning as exp");
                return Ok(SymbolicExpression::Lambda(exp.clone().try_into().unwrap()));
            } else {
                let lambda: Vec<SymbolicExpression> = exp[0].clone().try_into().unwrap();
                debug!("lambda exp: {:?}", lambda);
                debug!("evaluating call to lambda {:?}", lambda);
                let params = HashMap::new();
                let signature = lambda[1].clone();
                let mapped_exp = map_args(
                    SymbolicExpression::List(exp.clone()),
                    signature.clone(),
                    env,
                )
                .unwrap();
                let l_wrapper: Vec<SymbolicExpression> = mapped_exp.try_into().unwrap();
                let mapped_lambda: Vec<SymbolicExpression> =
                    l_wrapper[0].clone().try_into().unwrap();
                debug!("evaulating mapped lambda exp {:?}", l_wrapper);
                let proc = Proc {
                    body: mapped_lambda[2].clone(),
                    params,
                    env,
                    signature,
                };
                debug!(
                    "Invoking lambda {} last value in sym exp length: {}",
                    proc,
                    exp.len()
                );
                debug!("lambda found to have more expressions: {:?}", lambda);
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
                SymbolicExpression::Atom(p) => {
                    let exp = match arguements {
                        SymbolicExpression::Atom(_) => {
                            return Ok(exp.clone());
                        }
                        SymbolicExpression::List(ref vec) => {
                            debug!("found argument to lambda: {} arg: {}", exp, vec[0]);
                            vec[0].clone()
                        }
                        SymbolicExpression::Lambda(ref l) => SymbolicExpression::Lambda(l.clone()),
                    };

                    param_pairs.insert(exp.to_string(), SymbolicExpression::Atom(p));
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
                            SymbolicExpression::Lambda(ref l) => {
                                SymbolicExpression::Lambda(l.clone())
                            }
                        };

                        let symbol = p[i].clone();
                        param_pairs.insert(exp.to_string(), symbol);
                    }
                }
                SymbolicExpression::Lambda(p) => {
                    for i in 0..p.len() {
                        let exp = match arguements {
                            SymbolicExpression::Atom(_) => {
                                return Ok(exp.clone());
                            }
                            SymbolicExpression::List(ref vec) => {
                                debug!("found argument to lambda: {} arg: {}", exp, vec[i]);
                                vec[i].clone()
                            }
                            SymbolicExpression::Lambda(ref l) => {
                                SymbolicExpression::Lambda(l.clone())
                            }
                        };

                        let symbol = p[i].clone();
                        param_pairs.insert(exp.to_string(), symbol);
                    }
                }
            }
        }
        SymbolicExpression::Lambda(_) => todo!(),
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
