use log::debug;
use tailcall::tailcall;

use crate::{
    proc::ProcedureFn,
    types::{
        list::PairList, pair::Pair, Atom, Begin, Cond, Define, ExprKind, If, Lambda, Let, Number,
        Procedure, RLispBoolean, RLispNumber, Value,
    },
};
use std::{collections::HashMap, sync::Arc};

#[tailcall]
pub fn eval(
    expression: ExprKind,
    env: &HashMap<String, ProcedureFn>,
    symbol_definitions: &mut HashMap<String, Value>,
) -> Result<Value, String> {
    debug!("evaluating expression: {:?}", expression);
    match expression {
        ExprKind::Atom(atom) => match atom.as_ref() {
            Atom::Number(n) => Ok(Value::Number(rlisp_to_number(n))),
            Atom::Bool(b) => Ok(Value::Boolean(rlisp_to_bool(b))),
            Atom::Symbol(s) => {
                debug!("looking up symbol definition for {}", s);
                symbol_definitions
                    .get(s)
                    .cloned()
                    .ok_or_else(|| format!("undefined symbol: {}", s))
            }
        },
        ExprKind::Cond(cond_exp) => eval_cond(
            Arc::try_unwrap(cond_exp).unwrap_or_else(|arc| (*arc).clone()),
            env,
            symbol_definitions,
        ),
        ExprKind::Define(define) => eval_define(
            Arc::try_unwrap(define).unwrap_or_else(|arc| (*arc).clone()),
            env,
            symbol_definitions,
        ),
        ExprKind::If(if_expr) => eval_if(
            Arc::try_unwrap(if_expr).unwrap_or_else(|arc| (*arc).clone()),
            env,
            symbol_definitions,
        ),
        ExprKind::Let(let_expr) => eval_let(
            Arc::try_unwrap(let_expr).unwrap_or_else(|arc| (*arc).clone()),
            env,
            symbol_definitions,
        ),
        ExprKind::Begin(begin) => eval_begin(
            Arc::try_unwrap(begin).unwrap_or_else(|arc| (*arc).clone()),
            env,
            symbol_definitions,
        ),
        ExprKind::Lambda(lambda) => eval_lambda(
            Arc::try_unwrap(lambda).unwrap_or_else(|arc| (*arc).clone()),
            symbol_definitions,
        ),
        ExprKind::List(list) => eval_list(
            Arc::try_unwrap(list).unwrap_or_else(|arc| (*arc).clone()),
            env,
            symbol_definitions,
        ),
        ExprKind::Quote(quote) => Ok(expr_to_value(&quote.expr)),
        ExprKind::StringLiteral(s) => Ok(Value::String(s)),
        ExprKind::Pair(p) => eval_pair(
            Arc::try_unwrap(p).unwrap_or_else(|arc| (*arc).clone()),
            env,
            symbol_definitions,
        ),
    }
}

// Convert RLispNumber to Number
fn rlisp_to_number(n: &RLispNumber) -> Number {
    match n {
        RLispNumber::Int(i) => Number::Int(*i as i64),
        RLispNumber::Float(f) => Number::Float(*f as f64),
    }
}

// Convert RLispBoolean to bool
fn rlisp_to_bool(b: &RLispBoolean) -> bool {
    match b {
        RLispBoolean::True(_) => true,
        RLispBoolean::False(_) => false,
    }
}

// Convert ExprKind to Value (for quoted expressions)
fn expr_to_value(expr: &ExprKind) -> Value {
    match expr {
        ExprKind::Atom(atom) => match atom.as_ref() {
            Atom::Number(n) => Value::Number(rlisp_to_number(n)),
            Atom::Bool(b) => Value::Boolean(rlisp_to_bool(b)),
            Atom::Symbol(s) => Value::Symbol(s.clone()),
        },
        ExprKind::List(list) => {
            let elements: Vec<Value> = list.to_vec().iter().map(expr_to_value).collect();
            vec_to_list(elements)
        }
        ExprKind::StringLiteral(s) => Value::String(s.clone()),
        ExprKind::Pair(p) => {
            let car = p
                .car
                .as_ref()
                .map(|c| expr_to_value(c))
                .unwrap_or(Value::Nil);
            let cdr = p
                .cdr
                .as_ref()
                .map(|c| expr_to_value(c))
                .unwrap_or(Value::Nil);
            Value::Pair(Arc::new((car, cdr)))
        }
        ExprKind::Quote(q) => expr_to_value(&q.expr),
        _ => Value::Nil,
    }
}

// Convert Vec to proper list
fn vec_to_list(vals: Vec<Value>) -> Value {
    vals.into_iter()
        .rev()
        .fold(Value::Nil, |acc, val| Value::Pair(Arc::new((val, acc))))
}

#[tailcall]
fn eval_define(
    define: Define,
    env: &HashMap<String, ProcedureFn>,
    symbol_definitions: &mut HashMap<String, Value>,
) -> Result<Value, String> {
    let name = match define.name {
        ExprKind::Atom(atom) => match Arc::try_unwrap(atom).unwrap_or_else(|arc| (*arc).clone()) {
            Atom::Symbol(s) => s,
            _ => return Err("define: name must be a symbol".to_string()),
        },
        _ => return Err("define: name must be a symbol".to_string()),
    };

    let value = eval(define.body, env, symbol_definitions)?;
    symbol_definitions.insert(name, value);

    Ok(Value::Void)
}

#[tailcall]
fn eval_cond(
    cond_expr: Cond,
    env: &HashMap<String, ProcedureFn>,
    symbol_definitions: &mut HashMap<String, Value>,
) -> Result<Value, String> {
    if let ExprKind::List(tests) = cond_expr.test_exps {
        let tests_vec = tests.to_vec();
        for test in tests_vec.iter() {
            if let ExprKind::List(l) = test {
                if l.length() < 2 {
                    return Err("invalid test expression".to_string());
                }
                let test_exp = l.nth(0).ok_or("missing test expression")?.as_ref().clone();
                let test_res = eval(test_exp, env, symbol_definitions)?;

                if test_res.is_truthy() {
                    let result_expr = l
                        .nth(1)
                        .ok_or("missing result expression")?
                        .as_ref()
                        .clone();
                    return eval(result_expr, env, symbol_definitions);
                }
            } else {
                return Err("invalid test expression".to_string());
            }
        }
    }

    eval(cond_expr.else_expr, env, symbol_definitions)
}

#[tailcall]
fn eval_if(
    if_expr: If,
    env: &HashMap<String, ProcedureFn>,
    symbol_definitions: &mut HashMap<String, Value>,
) -> Result<Value, String> {
    debug!(
        "evaling test cond: {}, symbol defs: {:?}",
        if_expr.test_expr, symbol_definitions
    );
    let test_result = eval(if_expr.test_expr, env, symbol_definitions)?;

    // R7RS: only #f is falsy
    if test_result.is_truthy() {
        eval(if_expr.then_expr, env, symbol_definitions)
    } else {
        eval(if_expr.else_expr, env, symbol_definitions)
    }
}

#[tailcall]
fn eval_begin(
    begin: Begin,
    env: &HashMap<String, ProcedureFn>,
    symbol_definitions: &mut HashMap<String, Value>,
) -> Result<Value, String> {
    let mut result = Value::Void;

    for expr in begin.exprs {
        result = eval(expr, env, symbol_definitions)?;
    }

    Ok(result)
}

fn eval_lambda(
    lambda: Lambda,
    symbol_definitions: &HashMap<String, Value>,
) -> Result<Value, String> {
    debug!("returning lambda {:?}", lambda);

    // Extract parameter names
    let params = extract_param_names(&lambda.args)?;

    Ok(Value::Procedure(Arc::new(Procedure {
        params,
        body: lambda.body.clone(),
        env: symbol_definitions.clone(),
    })))
}

fn extract_param_names(args: &ExprKind) -> Result<Vec<String>, String> {
    match args {
        ExprKind::List(list) => {
            let mut names = Vec::new();
            for param in list.to_vec() {
                if let ExprKind::Atom(atom) = param {
                    if let Atom::Symbol(s) = atom.as_ref() {
                        names.push(s.clone());
                    } else {
                        return Err("lambda parameters must be symbols".to_string());
                    }
                } else {
                    return Err("lambda parameters must be symbols".to_string());
                }
            }
            Ok(names)
        }
        ExprKind::Atom(atom) => {
            if let Atom::Symbol(s) = atom.as_ref() {
                Ok(vec![s.clone()])
            } else {
                Err("lambda parameter must be a symbol".to_string())
            }
        }
        _ => Err("invalid lambda parameter specification".to_string()),
    }
}

#[tailcall]
fn eval_list(
    list: PairList,
    env: &HashMap<String, ProcedureFn>,
    symbol_definitions: &mut HashMap<String, Value>,
) -> Result<Value, String> {
    if list.is_empty() {
        return Ok(Value::Nil);
    }

    let args_vec = list.to_vec();
    let operator = &args_vec[0];

    // If the first element is a symbol, check if it's a procedure
    if let ExprKind::Atom(atom) = operator {
        if let Atom::Symbol(ref name) = **atom {
            // Check built-in procedures
            if let Some(proc) = env.get(name) {
                debug!("found proc: {}", name);
                let mut evaluated_args = Vec::new();
                for arg in args_vec[1..].iter() {
                    debug!("evaluating expression in list {:?}", arg);
                    let evaluated = eval(arg.clone(), env, symbol_definitions)?;
                    debug!("evaluated expression {:?}", evaluated);
                    evaluated_args.push(evaluated);
                }

                debug!("invoking proc {} with args {:?}", name, evaluated_args);
                let proc_res = proc(evaluated_args, symbol_definitions);
                debug!("result of proc {:?}", proc_res);
                return proc_res;
            }

            // Check user-defined procedures
            if let Some(Value::Procedure(proc)) = symbol_definitions.get(name).cloned() {
                debug!("found procedure {}", name);
                let mut evaluated_args = Vec::new();
                for arg in args_vec[1..].iter() {
                    let evaluated = eval(arg.clone(), env, symbol_definitions)?;
                    evaluated_args.push(evaluated);
                }

                return apply_procedure(&proc, evaluated_args, env, symbol_definitions);
            }
        }
    }

    // If we get here, evaluate each element in the list
    let mut evaluated_args = Vec::new();
    for arg in args_vec.iter() {
        let evaluated = eval(arg.clone(), env, symbol_definitions)?;
        evaluated_args.push(evaluated);
    }
    debug!("resulting list {:?}", evaluated_args);
    Ok(vec_to_list(evaluated_args))
}

fn apply_procedure(
    proc: &Procedure,
    args: Vec<Value>,
    env: &HashMap<String, ProcedureFn>,
    outer_symbols: &mut HashMap<String, Value>,
) -> Result<Value, String> {
    if args.len() != proc.params.len() {
        return Err(format!(
            "procedure expects {} arguments, got {}",
            proc.params.len(),
            args.len()
        ));
    }

    // Create local scope: start with captured env, overlay current symbols (for recursion),
    // then add parameters
    let mut local_symbols = proc.env.clone();
    local_symbols.extend(outer_symbols.clone()); // Allow recursion by including current definitions
    for (name, value) in proc.params.iter().zip(args.into_iter()) {
        local_symbols.insert(name.clone(), value);
    }

    eval(proc.body.clone(), env, &mut local_symbols)
}

// Helper for map/filter: apply a procedure to a single argument
pub fn apply_proc_to_arg(
    proc: &Procedure,
    arg: Value,
    env: &HashMap<String, ProcedureFn>,
    symbol_definitions: &mut HashMap<String, Value>,
) -> Result<Value, String> {
    apply_procedure(proc, vec![arg], env, symbol_definitions)
}

#[tailcall]
fn eval_let(
    let_expr: Let,
    env: &HashMap<String, ProcedureFn>,
    symbol_definitions: &mut HashMap<String, Value>,
) -> Result<Value, String> {
    debug!("expression found to be let {:?}", let_expr);
    let mut local_symbols = symbol_definitions.clone();

    if let ExprKind::List(defines) = let_expr.declerations {
        for def in defines.to_vec().iter() {
            let _ = eval(def.clone(), env, &mut local_symbols);
        }

        debug!("current symbol table: {:?}", &local_symbols);
        if let ExprKind::List(_) = let_expr.proc_call.clone() {
            let res = eval(let_expr.proc_call, env, &mut local_symbols)?;
            return Ok(res);
        }
    }

    Ok(Value::Nil)
}

fn eval_pair(
    pair: Pair<ExprKind>,
    env: &HashMap<String, ProcedureFn>,
    symbol_definitions: &mut HashMap<String, Value>,
) -> Result<Value, String> {
    let car = pair
        .car
        .as_ref()
        .map(|c| eval(c.as_ref().clone(), env, symbol_definitions))
        .transpose()?
        .unwrap_or(Value::Nil);
    let cdr = pair
        .cdr
        .as_ref()
        .map(|c| eval(c.as_ref().clone(), env, symbol_definitions))
        .transpose()?
        .unwrap_or(Value::Nil);

    Ok(Value::Pair(Arc::new((car, cdr))))
}
