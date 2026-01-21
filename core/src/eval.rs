use log::debug;

use crate::{
    proc::ProcedureFn,
    types::{Procedure, SchemeList, Value},
};
use std::{collections::HashMap, sync::Arc};

/// Evaluate a Value expression (R7RS-style evaluator)
pub fn eval_value(
    expr: Value,
    env: &HashMap<String, ProcedureFn>,
    defs: &mut HashMap<String, Value>,
) -> Result<Value, String> {
    debug!("eval_value: {:?}", expr);
    match expr {
        // Self-evaluating forms (R7RS 4.1.2)
        Value::Number(_)
        | Value::Boolean(_)
        | Value::Char(_)
        | Value::String(_)
        | Value::Procedure(_)
        | Value::Void => Ok(expr),

        // Symbol lookup (R7RS 4.1.1)
        Value::Symbol(ref name) => defs
            .get(name)
            .cloned()
            .ok_or_else(|| format!("undefined symbol: {}", name)),

        // Empty list is an error when evaluated (R7RS 4.1.3)
        Value::Nil => Err("empty list () is not a valid expression".to_string()),

        // List evaluation - check for special forms or procedure call
        Value::List(list) => eval_value_list(&list, env, defs),

        // Pair evaluation
        Value::Pair(p) => {
            let car = eval_value(p.0.clone(), env, defs)?;
            let cdr = eval_value(p.1.clone(), env, defs)?;
            Ok(Value::Pair(Arc::new((car, cdr))))
        }
    }
}

/// Evaluate a list expression - handles special forms and procedure calls
fn eval_value_list(
    list: &SchemeList,
    env: &HashMap<String, ProcedureFn>,
    defs: &mut HashMap<String, Value>,
) -> Result<Value, String> {
    if list.is_empty() {
        return Err("empty list () is not a valid expression".to_string());
    }

    let first = list.car().ok_or("invalid list structure")?;

    // Check for special forms
    if let Value::Symbol(ref name) = first {
        match name.as_str() {
            "quote" => return eval_value_quote(list),
            "define" => return eval_value_define(list, env, defs),
            "if" => return eval_value_if(list, env, defs),
            "lambda" => return eval_value_lambda(list, defs),
            "let" => return eval_value_let(list, env, defs),
            "begin" => return eval_value_begin(list, env, defs),
            "cond" => return eval_value_cond(list, env, defs),
            "set!" => return eval_value_set(list, env, defs),
            _ => {}
        }
    }

    // Not a special form - it's a procedure call
    eval_value_application(list, env, defs)
}

/// (quote <datum>) -> datum
fn eval_value_quote(list: &SchemeList) -> Result<Value, String> {
    if list.length() != 2 {
        return Err("quote requires exactly one argument".to_string());
    }
    Ok(list.nth(1).unwrap())
}

/// (define <name> <expr>) or (define (<name> <params>...) <body>)
fn eval_value_define(
    list: &SchemeList,
    env: &HashMap<String, ProcedureFn>,
    defs: &mut HashMap<String, Value>,
) -> Result<Value, String> {
    if list.length() < 3 {
        return Err("define requires at least 2 arguments".to_string());
    }

    let second = list.nth(1).ok_or("define: missing name")?;

    match second {
        Value::Symbol(name) => {
            // Simple define: (define name expr)
            let body = list.nth(2).ok_or("define: missing body")?;
            let value = eval_value(body, env, defs)?;
            defs.insert(name, value);
            Ok(Value::Void)
        }
        Value::List(name_and_params) => {
            // Function shorthand: (define (name params...) body)
            let name = match name_and_params.car() {
                Some(Value::Symbol(n)) => n.clone(),
                _ => return Err("define: function name must be a symbol".to_string()),
            };

            // Build params list from cdr of name_and_params
            let params: Vec<String> = name_and_params
                .cdr()
                .map(|cdr| {
                    cdr.to_vec()
                        .into_iter()
                        .filter_map(|v| {
                            if let Value::Symbol(s) = v {
                                Some(s)
                            } else {
                                None
                            }
                        })
                        .collect()
                })
                .unwrap_or_default();

            let body = list.nth(2).ok_or("define: missing body")?;

            let proc = Value::Procedure(Arc::new(Procedure {
                params,
                body: Box::new(body),
                env: defs.clone(),
            }));

            defs.insert(name, proc);
            Ok(Value::Void)
        }
        _ => Err("define: first argument must be a symbol or list".to_string()),
    }
}

/// (if <test> <consequent> <alternate>)
fn eval_value_if(
    list: &SchemeList,
    env: &HashMap<String, ProcedureFn>,
    defs: &mut HashMap<String, Value>,
) -> Result<Value, String> {
    if list.length() < 3 {
        return Err("if requires at least 2 arguments".to_string());
    }

    let test = list.nth(1).ok_or("if: missing test")?;
    let test_result = eval_value(test, env, defs)?;

    if test_result.is_truthy() {
        let consequent = list.nth(2).ok_or("if: missing consequent")?;
        eval_value(consequent, env, defs)
    } else if list.length() > 3 {
        let alternate = list.nth(3).ok_or("if: missing alternate")?;
        eval_value(alternate, env, defs)
    } else {
        Ok(Value::Void)
    }
}

/// (lambda (<params>...) <body>)
fn eval_value_lambda(list: &SchemeList, defs: &HashMap<String, Value>) -> Result<Value, String> {
    if list.length() < 3 {
        return Err("lambda requires at least 2 arguments".to_string());
    }

    let params_value = list.nth(1).ok_or("lambda: missing parameters")?;
    let params = extract_value_params(&params_value)?;
    let body = list.nth(2).ok_or("lambda: missing body")?;

    Ok(Value::Procedure(Arc::new(Procedure {
        params,
        body: Box::new(body),
        env: defs.clone(),
    })))
}

/// Extract parameter names from a Value (list of symbols or single symbol)
fn extract_value_params(params: &Value) -> Result<Vec<String>, String> {
    match params {
        Value::List(list) => {
            let mut names = Vec::new();
            for v in list.to_vec() {
                if let Value::Symbol(s) = v {
                    names.push(s);
                } else {
                    return Err("lambda parameters must be symbols".to_string());
                }
            }
            Ok(names)
        }
        Value::Symbol(s) => Ok(vec![s.clone()]),
        Value::Nil => Ok(vec![]),
        _ => Err("lambda parameters must be a list or symbol".to_string()),
    }
}

/// (let ((<name> <expr>)...) <body>)
fn eval_value_let(
    list: &SchemeList,
    env: &HashMap<String, ProcedureFn>,
    defs: &mut HashMap<String, Value>,
) -> Result<Value, String> {
    if list.length() < 3 {
        return Err("let requires at least 2 arguments".to_string());
    }

    let bindings_value = list.nth(1).ok_or("let: missing bindings")?;
    let body = list.nth(2).ok_or("let: missing body")?;

    let mut local_defs = defs.clone();

    // Process bindings
    if let Value::List(bindings) = bindings_value {
        for binding in bindings.to_vec() {
            if let Value::List(pair) = binding {
                if pair.length() != 2 {
                    return Err("let binding must be (name expr)".to_string());
                }
                let name = match pair.nth(0) {
                    Some(Value::Symbol(s)) => s,
                    _ => return Err("let binding name must be a symbol".to_string()),
                };
                let expr = pair.nth(1).ok_or("let binding missing expression")?;
                let value = eval_value(expr, env, defs)?;
                local_defs.insert(name, value);
            } else {
                return Err("let binding must be a list".to_string());
            }
        }
    } else {
        return Err("let bindings must be a list".to_string());
    }

    eval_value(body, env, &mut local_defs)
}

/// (begin <expr>...)
fn eval_value_begin(
    list: &SchemeList,
    env: &HashMap<String, ProcedureFn>,
    defs: &mut HashMap<String, Value>,
) -> Result<Value, String> {
    let mut result = Value::Void;

    // Skip "begin" symbol, evaluate rest
    let exprs = list.to_vec();
    for expr in exprs.into_iter().skip(1) {
        result = eval_value(expr, env, defs)?;
    }

    Ok(result)
}

/// (cond (<test> <expr>)... (else <expr>))
fn eval_value_cond(
    list: &SchemeList,
    env: &HashMap<String, ProcedureFn>,
    defs: &mut HashMap<String, Value>,
) -> Result<Value, String> {
    let clauses = list.to_vec();

    for clause in clauses.into_iter().skip(1) {
        if let Value::List(clause_list) = clause {
            if clause_list.length() < 2 {
                return Err("cond clause must have test and expression".to_string());
            }

            let test = clause_list.nth(0).ok_or("cond: missing test")?;

            // Check for else clause
            if let Value::Symbol(ref s) = test {
                if s == "else" {
                    let result = clause_list.nth(1).ok_or("cond: missing else expression")?;
                    return eval_value(result, env, defs);
                }
            }

            let test_result = eval_value(test, env, defs)?;
            if test_result.is_truthy() {
                let result = clause_list
                    .nth(1)
                    .ok_or("cond: missing result expression")?;
                return eval_value(result, env, defs);
            }
        } else {
            return Err("cond clause must be a list".to_string());
        }
    }

    Ok(Value::Void)
}

/// (set! <name> <expr>)
fn eval_value_set(
    list: &SchemeList,
    env: &HashMap<String, ProcedureFn>,
    defs: &mut HashMap<String, Value>,
) -> Result<Value, String> {
    if list.length() != 3 {
        return Err("set! requires exactly 2 arguments".to_string());
    }

    let name = match list.nth(1) {
        Some(Value::Symbol(s)) => s,
        _ => return Err("set!: first argument must be a symbol".to_string()),
    };

    if !defs.contains_key(&name) {
        return Err(format!("set!: undefined variable: {}", name));
    }

    let expr = list.nth(2).ok_or("set!: missing expression")?;
    let value = eval_value(expr, env, defs)?;
    defs.insert(name, value);

    Ok(Value::Void)
}

/// Evaluate a procedure application
fn eval_value_application(
    list: &SchemeList,
    env: &HashMap<String, ProcedureFn>,
    defs: &mut HashMap<String, Value>,
) -> Result<Value, String> {
    let items = list.to_vec();
    if items.is_empty() {
        return Err("empty application".to_string());
    }

    let operator = &items[0];

    // Evaluate all arguments first
    let mut evaluated_args = Vec::new();
    for arg in items[1..].iter() {
        evaluated_args.push(eval_value(arg.clone(), env, defs)?);
    }

    // Check if operator is a symbol referencing a built-in or defined procedure
    if let Value::Symbol(ref name) = operator {
        // Check built-in procedures
        if let Some(proc) = env.get(name) {
            debug!("calling built-in: {}", name);
            return proc(evaluated_args, defs);
        }

        // Check user-defined procedures
        if let Some(Value::Procedure(proc)) = defs.get(name).cloned() {
            debug!("calling user-defined: {}", name);
            return apply_value_procedure(&proc, evaluated_args, env, defs);
        }

        return Err(format!("undefined procedure: {}", name));
    }

    // Evaluate the operator expression
    let proc_value = eval_value(operator.clone(), env, defs)?;

    if let Value::Procedure(proc) = proc_value {
        apply_value_procedure(&proc, evaluated_args, env, defs)
    } else {
        Err(format!("not a procedure: {:?}", proc_value))
    }
}

/// Apply a user-defined procedure to arguments
fn apply_value_procedure(
    proc: &Procedure,
    args: Vec<Value>,
    env: &HashMap<String, ProcedureFn>,
    outer_defs: &mut HashMap<String, Value>,
) -> Result<Value, String> {
    if args.len() != proc.params.len() {
        return Err(format!(
            "procedure expects {} arguments, got {}",
            proc.params.len(),
            args.len()
        ));
    }

    // Create local scope
    let mut local_defs = proc.env.clone();
    local_defs.extend(outer_defs.clone());

    for (name, value) in proc.params.iter().zip(args.into_iter()) {
        local_defs.insert(name.clone(), value);
    }

    // Evaluate the body with the new scope
    eval_value((*proc.body).clone(), env, &mut local_defs)
}

/// Helper for map/filter: apply a procedure to a single argument
pub fn apply_proc_to_arg(
    proc: &Procedure,
    arg: Value,
    env: &HashMap<String, ProcedureFn>,
    defs: &mut HashMap<String, Value>,
) -> Result<Value, String> {
    apply_value_procedure(proc, vec![arg], env, defs)
}

/// Convert Vec to proper list Value
pub fn vec_to_list(vals: Vec<Value>) -> Value {
    vals.into_iter()
        .rev()
        .fold(Value::Nil, |acc, val| Value::Pair(Arc::new((val, acc))))
}
