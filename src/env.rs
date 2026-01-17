use crate::{
    eval::apply_proc_to_arg,
    op::ValueNumericOps,
    types::{Number, Value},
};
use std::{collections::HashMap, sync::Arc};

pub type ProcedureFn =
    Box<dyn Fn(Vec<Value>, &mut HashMap<String, Value>) -> Result<Value, String>>;

pub fn std_const_exp() -> HashMap<String, Value> {
    let mut const_exps = HashMap::new();
    const_exps.insert("pi".to_string(), Value::Number(Number::Float(3.141592653589793)));
    const_exps
}

pub fn std_env() -> HashMap<String, ProcedureFn> {
    let mut env: HashMap<String, ProcedureFn> = HashMap::new();

    // Arithmetic operations
    env.insert(
        "+".to_string(),
        Box::new(|args, _| {
            if args.len() < 2 {
                return Err("+ requires at least two arguments".to_string());
            }
            args[0].add(&args[1])
        }),
    );

    env.insert(
        "-".to_string(),
        Box::new(|args, _| {
            if args.len() < 2 {
                return Err("- requires at least two arguments".to_string());
            }
            args[0].sub(&args[1])
        }),
    );

    env.insert(
        "*".to_string(),
        Box::new(|args, _| {
            if args.len() < 2 {
                return Err("* requires at least two arguments".to_string());
            }
            args[0].mul(&args[1])
        }),
    );

    env.insert(
        "/".to_string(),
        Box::new(|args, _| {
            if args.len() < 2 {
                return Err("/ requires at least two arguments".to_string());
            }
            args[0].div(&args[1])
        }),
    );

    // Comparison operations
    env.insert(
        "<".to_string(),
        Box::new(|args, _| {
            if args.len() < 2 {
                return Err("< requires at least two arguments".to_string());
            }
            match (&args[0], &args[1]) {
                (Value::Number(l), Value::Number(r)) => Ok(Value::Boolean(l < r)),
                _ => Err("< requires numeric arguments".to_string()),
            }
        }),
    );

    env.insert(
        ">".to_string(),
        Box::new(|args, _| {
            if args.len() < 2 {
                return Err("> requires at least two arguments".to_string());
            }
            match (&args[0], &args[1]) {
                (Value::Number(l), Value::Number(r)) => Ok(Value::Boolean(l > r)),
                _ => Err("> requires numeric arguments".to_string()),
            }
        }),
    );

    env.insert(
        "=".to_string(),
        Box::new(|args, _| {
            if args.len() < 2 {
                return Err("= requires at least two arguments".to_string());
            }
            Ok(Value::Boolean(args[0] == args[1]))
        }),
    );

    // Boolean operations
    env.insert(
        "and".to_string(),
        Box::new(|args, _| {
            if args.len() < 2 {
                return Err("and requires at least two arguments".to_string());
            }
            Ok(Value::Boolean(args[0].is_truthy() && args[1].is_truthy()))
        }),
    );

    env.insert(
        "or".to_string(),
        Box::new(|args, _| {
            if args.len() < 2 {
                return Err("or requires at least two arguments".to_string());
            }
            Ok(Value::Boolean(args[0].is_truthy() || args[1].is_truthy()))
        }),
    );

    env.insert(
        "not".to_string(),
        Box::new(|args, _| {
            if args.is_empty() {
                return Err("not requires one argument".to_string());
            }
            Ok(Value::Boolean(!args[0].is_truthy()))
        }),
    );

    // Type predicates
    env.insert(
        "number?".to_string(),
        Box::new(|args, _| {
            if args.is_empty() {
                return Err("number? requires one argument".to_string());
            }
            Ok(Value::Boolean(matches!(args[0], Value::Number(_))))
        }),
    );

    env.insert(
        "boolean?".to_string(),
        Box::new(|args, _| {
            if args.is_empty() {
                return Err("boolean? requires one argument".to_string());
            }
            Ok(Value::Boolean(matches!(args[0], Value::Boolean(_))))
        }),
    );

    env.insert(
        "string?".to_string(),
        Box::new(|args, _| {
            if args.is_empty() {
                return Err("string? requires one argument".to_string());
            }
            Ok(Value::Boolean(matches!(args[0], Value::String(_))))
        }),
    );

    env.insert(
        "procedure?".to_string(),
        Box::new(|args, _| {
            if args.is_empty() {
                return Err("procedure? requires one argument".to_string());
            }
            Ok(Value::Boolean(matches!(args[0], Value::Procedure(_))))
        }),
    );

    env.insert(
        "pair?".to_string(),
        Box::new(|args, _| {
            if args.is_empty() {
                return Err("pair? requires one argument".to_string());
            }
            Ok(Value::Boolean(matches!(args[0], Value::Pair(_))))
        }),
    );

    env.insert(
        "null?".to_string(),
        Box::new(|args, _| {
            if args.is_empty() {
                return Err("null? requires one argument".to_string());
            }
            Ok(Value::Boolean(matches!(args[0], Value::Nil)))
        }),
    );

    env.insert(
        "list?".to_string(),
        Box::new(|args, _| {
            if args.is_empty() {
                return Err("list? requires one argument".to_string());
            }
            Ok(Value::Boolean(is_list(&args[0])))
        }),
    );

    env.insert(
        "symbol?".to_string(),
        Box::new(|args, _| {
            if args.is_empty() {
                return Err("symbol? requires one argument".to_string());
            }
            Ok(Value::Boolean(matches!(args[0], Value::Symbol(_))))
        }),
    );

    // List operations
    env.insert(
        "cons".to_string(),
        Box::new(|args, _| {
            if args.len() != 2 {
                return Err("cons requires 2 arguments".to_string());
            }
            Ok(Value::Pair(Arc::new((args[0].clone(), args[1].clone()))))
        }),
    );

    env.insert(
        "car".to_string(),
        Box::new(|args, _| {
            if args.is_empty() {
                return Err("car requires one argument".to_string());
            }
            match &args[0] {
                Value::Pair(p) => Ok(p.0.clone()),
                Value::Nil => Err("car: empty list".to_string()),
                _ => Err("car: argument must be a pair".to_string()),
            }
        }),
    );

    env.insert(
        "cdr".to_string(),
        Box::new(|args, _| {
            if args.is_empty() {
                return Err("cdr requires one argument".to_string());
            }
            match &args[0] {
                Value::Pair(p) => Ok(p.1.clone()),
                Value::Nil => Err("cdr: empty list".to_string()),
                _ => Err("cdr: argument must be a pair".to_string()),
            }
        }),
    );

    env.insert(
        "list".to_string(),
        Box::new(|args, _| Ok(vec_to_list(args))),
    );

    env.insert(
        "length".to_string(),
        Box::new(|args, _| {
            if args.is_empty() {
                return Err("length requires one argument".to_string());
            }
            let len = list_length(&args[0])?;
            Ok(Value::Number(Number::Int(len)))
        }),
    );

    env.insert(
        "append".to_string(),
        Box::new(|args, _| {
            if args.len() < 2 {
                return Err("append requires at least two arguments".to_string());
            }
            let mut result: Vec<Value> = Vec::new();
            for arg in args.iter() {
                let elements = list_to_vec(arg)?;
                result.extend(elements);
            }
            Ok(vec_to_list(result))
        }),
    );

    // I/O
    env.insert(
        "display".to_string(),
        Box::new(|args, _| {
            for arg in args.iter() {
                print!("{}", arg);
            }
            Ok(Value::Void)
        }),
    );

    env.insert(
        "newline".to_string(),
        Box::new(|_, _| {
            println!();
            Ok(Value::Void)
        }),
    );

    // Exactness predicates
    env.insert(
        "exact?".to_string(),
        Box::new(|args, _| {
            if args.is_empty() {
                return Err("exact? requires one argument".to_string());
            }
            Ok(Value::Boolean(matches!(args[0], Value::Number(Number::Int(_)))))
        }),
    );

    env.insert(
        "inexact?".to_string(),
        Box::new(|args, _| {
            if args.is_empty() {
                return Err("inexact? requires one argument".to_string());
            }
            Ok(Value::Boolean(matches!(args[0], Value::Number(Number::Float(_)))))
        }),
    );

    // Higher-order functions
    env.insert(
        "map".to_string(),
        Box::new(|args, sd| {
            if args.len() != 2 {
                return Err("map requires 2 arguments".to_string());
            }
            let proc = match &args[0] {
                Value::Procedure(p) => p.clone(),
                _ => return Err("map: first argument must be a procedure".to_string()),
            };
            let list_elements = list_to_vec(&args[1])?;
            let env = crate::env::std_env();
            
            let mut results = Vec::new();
            for elem in list_elements {
                let result = apply_proc_to_arg(&proc, elem, &env, sd)?;
                results.push(result);
            }
            Ok(vec_to_list(results))
        }),
    );

    env.insert(
        "filter".to_string(),
        Box::new(|args, sd| {
            if args.len() != 2 {
                return Err("filter requires 2 arguments".to_string());
            }
            let proc = match &args[0] {
                Value::Procedure(p) => p.clone(),
                _ => return Err("filter: first argument must be a procedure".to_string()),
            };
            let list_elements = list_to_vec(&args[1])?;
            let env = crate::env::std_env();
            
            let mut results = Vec::new();
            for elem in list_elements {
                let test_result = apply_proc_to_arg(&proc, elem.clone(), &env, sd)?;
                if test_result.is_truthy() {
                    results.push(elem);
                }
            }
            Ok(vec_to_list(results))
        }),
    );

    env
}

// Helper: check if a Value is a proper list
fn is_list(val: &Value) -> bool {
    match val {
        Value::Nil => true,
        Value::Pair(p) => is_list(&p.1),
        _ => false,
    }
}

// Helper: convert Value list to Vec
fn list_to_vec(val: &Value) -> Result<Vec<Value>, String> {
    let mut result = Vec::new();
    let mut current = val.clone();
    loop {
        match current {
            Value::Nil => break,
            Value::Pair(p) => {
                result.push(p.0.clone());
                current = p.1.clone();
            }
            _ => return Err("not a proper list".to_string()),
        }
    }
    Ok(result)
}

// Helper: convert Vec to Value list
fn vec_to_list(vals: Vec<Value>) -> Value {
    vals.into_iter().rev().fold(Value::Nil, |acc, val| {
        Value::Pair(Arc::new((val, acc)))
    })
}

// Helper: get list length
fn list_length(val: &Value) -> Result<i64, String> {
    let mut count = 0i64;
    let mut current = val.clone();
    loop {
        match current {
            Value::Nil => break,
            Value::Pair(p) => {
                count += 1;
                current = p.1.clone();
            }
            _ => return Err("not a proper list".to_string()),
        }
    }
    Ok(count)
}
