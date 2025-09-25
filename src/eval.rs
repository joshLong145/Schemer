use log::debug;

use crate::{
    proc::Eval, types::{
        Atom, Begin, Define, ExprKind, If, Lambda, Let, List, RLispBoolean
    }
};
use std::{collections::HashMap, sync::Arc};

type ProcedureFn = Box<dyn Fn(Vec<ExprKind>, &mut HashMap<String, ExprKind>) -> Result<ExprKind, String>>;

pub fn eval(
    expression: ExprKind,
    env: &HashMap<String, ProcedureFn>,
    symbol_definitions: &mut HashMap<String, ExprKind>,
) -> Result<ExprKind, String> {
    debug!("evaluating expression: {}", expression);
    match expression {
        ExprKind::Atom(atom) => {
            match *atom {
                Atom::Number(_) | Atom::Bool(_) => Ok(ExprKind::Atom(atom)),
                Atom::Symbol(ref s) => {
                    debug!("looking up symbol definition for {}", s);
                    symbol_definitions
                        .get(s)
                        .cloned()
                        .ok_or_else(|| format!("undefined symbol: {}", s))
                }
            }
        }
        ExprKind::Define(define) => eval_define(Arc::try_unwrap(define).unwrap_or_else(|arc| (*arc).clone()), env, symbol_definitions),
        ExprKind::If(if_expr) => eval_if(Arc::try_unwrap(if_expr).unwrap_or_else(|arc| (*arc).clone()), env, symbol_definitions),
        ExprKind::Let(let_expr) => eval_let(Arc::try_unwrap(let_expr).unwrap_or_else(|arc| (*arc).clone()), env, symbol_definitions),
        ExprKind::Begin(begin) => eval_begin(Arc::try_unwrap(begin).unwrap_or_else(|arc| (*arc).clone()), env, symbol_definitions),
        ExprKind::Lambda(lambda) => eval_lambda(Arc::try_unwrap(lambda).unwrap_or_else(|arc| (*arc).clone()), env, symbol_definitions),
        ExprKind::List(list) => eval_list(Arc::try_unwrap(list).unwrap_or_else(|arc| (*arc).clone()), env, symbol_definitions),
        ExprKind::Quote(quote) => Ok(ExprKind::Quote(quote)),
        ExprKind::StringLiteral(s) => Ok(ExprKind::StringLiteral(s)),
    }
}

fn eval_define(
    define: Define,
    env: &HashMap<String, ProcedureFn>,
    symbol_definitions: &mut HashMap<String, ExprKind>,
) -> Result<ExprKind, String> {
    let name = match define.name {
        ExprKind::Atom(atom) => match Arc::try_unwrap(atom).unwrap_or_else(|arc| (*arc).clone()) {
            Atom::Symbol(s) => s,
            _ => return Err("define: name must be a symbol".to_string()),
        },
        _ => return Err("define: name must be a symbol".to_string()),
    };

    let value = eval(define.body, env, symbol_definitions)?;
    symbol_definitions.insert(name.clone(), value);

    Ok(ExprKind::Atom(Arc::new(Atom::Symbol(name))))
}

fn eval_if(
    if_expr: If,
    env: &HashMap<String, ProcedureFn>,
    symbol_definitions: &mut HashMap<String, ExprKind>,
) -> Result<ExprKind, String> {
    debug!("evaling test cond: {}, symbol defs: {:?}", if_expr.test_expr, symbol_definitions);
    let test_expr = resolve_symbol_if_present(&if_expr.test_expr, symbol_definitions, env);
    let test_result = eval(test_expr, env, symbol_definitions)?;

    match test_result {
        ExprKind::Atom(atom) => match *atom {
            Atom::Bool(RLispBoolean::True(_)) => eval(if_expr.then_expr, env, symbol_definitions),
            Atom::Bool(RLispBoolean::False(_)) => eval(if_expr.else_expr, env, symbol_definitions),
            _ => Err("test expression must evaluate to a boolean".to_string()),
        },
        _ => Err("test expression must evaluate to a boolean".to_string()),
    }
}

fn eval_begin(
    begin: Begin,
    env: &HashMap<String, ProcedureFn>,
    symbol_definitions: &mut HashMap<String, ExprKind>,
) -> Result<ExprKind, String> {
    let mut result = ExprKind::Atom(Arc::new(Atom::Symbol("()".to_string())));

    for expr in begin.exprs {
        result = eval(expr, env, symbol_definitions)?;
    }

    Ok(result)
}

fn eval_lambda(
    lambda: Lambda,
    _: &HashMap<String, ProcedureFn>,
    _: &mut HashMap<String, ExprKind>,
) -> Result<ExprKind, String> {
    debug!("returning lambda {:?}", lambda);
    Ok(ExprKind::Lambda(Arc::new(lambda)))
}

fn eval_list(
    list: List,
    env: &HashMap<String, ProcedureFn>,
    symbol_definitions: &mut HashMap<String, ExprKind>,
) -> Result<ExprKind, String> {
    if list.args.is_empty() {
        return Ok(ExprKind::List(Arc::new(list)));
    }

    let mut evaluated_args = Vec::new();
    let operator = &list.args[0];

    // If the first element is a symbol, check if it's a procedure
    if let ExprKind::Atom(atom) = operator {
        if let Atom::Symbol(ref name) = **atom {
            if let Some(proc) = env.get(name) {
                debug!("found proc: {}", name);
                let mut offset = 0;
                // Evaluate all arguments
                for arg in list.args[1..].iter() {
                    debug!("evaluating expression in list {}", arg);
                    let evaluated = eval(arg.clone(), env, symbol_definitions)?;
                    debug!("evaluated expression {}", evaluated);
                    evaluated_args.insert(offset, evaluated);
                    offset += 1;
                }

                debug!("invoking proc {} with args {:?}", name, evaluated_args);
                return proc(evaluated_args, symbol_definitions);
            } else if let Some(def) = symbol_definitions.get(name) {
                debug!("found symbol definition {} definition {}", name, def);

                if def.is_proc() {
                    debug!("found expression to be procedure call {}", def);
                    let mut param_evals: Vec<ExprKind> = Vec::new();
                    for e in list.args[1..list.args.len()].iter() {
                        let param_eval = eval(e.to_owned(), env, &mut symbol_definitions.clone())?;
                        param_evals.push(param_eval);
                    }

                    let proc = ExprKind::to_proc(def, param_evals, env).unwrap();
                    let res = proc.proc_eval(symbol_definitions).unwrap();
                    debug!("procedure result: {}", res);
                    return Ok(res);
                }
            }
        }
    }

    // If we get here, evaluate each element in the list
    for arg in list.args.iter() {
        let evaluated = eval(arg.clone(), env, symbol_definitions)?;
        evaluated_args.push(evaluated);
    }
    debug!("resulting list {:?}", evaluated_args);
    Ok(ExprKind::List(Arc::new(List {
        args: evaluated_args,
        object_id: list.object_id,
    })))
}


fn eval_let(
    let_expr: Let,
    env: &HashMap<String, ProcedureFn>,
    symbol_definitions: &mut HashMap<String, ExprKind>,
) -> Result<ExprKind, String> {
    debug!("expression found to be let {:?}", let_expr);
    let mut local_symbols = HashMap::new();
    local_symbols.extend(symbol_definitions.clone());

    if let ExprKind::List(defines) = let_expr.declerations {
        for def in defines.args.iter() {
            let _ = eval(def.clone(), env, &mut local_symbols);
        }

        debug!("current symbol table: {:?}", &mut local_symbols);
        if let ExprKind::List(_) = let_expr.proc_call.clone() {
            let res = eval(let_expr.proc_call, &env, &mut local_symbols)?;
            return Ok(res);
        }
    }

    Ok(ExprKind::List(Arc::new(List {
        args: vec![],
        object_id: 0,
    })))
}



pub fn resolve_symbol_if_present(
    expr: &ExprKind,
    symbol_definitions: &mut HashMap<String, ExprKind>,
    env: &HashMap<String, ProcedureFn>,
) -> ExprKind {
    match expr {
        ExprKind::Atom(atom) => match atom.as_ref() {
            Atom::Symbol(s) => {
                symbol_definitions.get(s).cloned().unwrap_or_else(|| expr.clone())
            }
            _ => expr.clone(),
        },
        ExprKind::List(list) => ExprKind::List(Arc::new(List {
            args: list
                .args
                .iter()
                .map(|e| resolve_symbol_if_present(e, symbol_definitions, env))
                .collect(),
            object_id: list.object_id,
        })),
        ExprKind::Lambda(lambda) => ExprKind::Lambda(Arc::new(Lambda {
            args: lambda.args.clone(),
            body: resolve_symbol_if_present(&lambda.body, symbol_definitions, env),
            object_id: lambda.object_id,
        })),
        _ => expr.clone(),
    }
}
