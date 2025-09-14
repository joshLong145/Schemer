use std::{collections::{HashMap, VecDeque}, sync::Arc};

use schemer::{
    env::std_env,
    eval::eval,
    parser::{parse, read_from_tokens},
    proc::ProcedureFn,
    types::{Atom, ExprKind, List, Quote, RLispBoolean, RLispNumber},
};

fn setup_logging() {
    pretty_env_logger::try_init().unwrap_or(());
}

#[test]
fn basic_parse_and_eval() {
    setup_logging();

    let mut exp_map: HashMap<String, VecDeque<String>> = HashMap::new();
    let mut token_map = parse("(begin (+ 1 1))".to_string(), &mut exp_map);
    let exp: ExprKind = read_from_tokens(&mut token_map).unwrap().into();

    let env: HashMap<String, ProcedureFn> = std_env();
    let mut symbol_definitions: HashMap<String, ExprKind> = HashMap::new();
    let res = eval(exp, &env, &mut symbol_definitions).unwrap();

    assert_eq!(
        res,
        ExprKind::Atom(Arc::new(Atom::Number(RLispNumber::Int(2))))
    );
}

#[test]
fn parse_and_eval_nested_operations() {
    setup_logging();

    let mut exp_map: HashMap<String, VecDeque<String>> = HashMap::new();
    let mut token_map = parse("(begin (+ (+ 1 1) (+ 1 1)))".to_string(), &mut exp_map);
    let exp: ExprKind = read_from_tokens(&mut token_map).unwrap().into();

    let env = std_env();
    let mut symbol_definitions: HashMap<String, ExprKind> = HashMap::new();
    let res = eval(exp, &env, &mut symbol_definitions).unwrap();

    assert_eq!(
        res,
        ExprKind::Atom(Arc::new(Atom::Number(RLispNumber::Int(4))))
    );
}

#[test]
fn parse_and_eval_var_declare_and_resolve_for_proc() {
    setup_logging();

    let mut exp_map: HashMap<String, VecDeque<String>> = HashMap::new();
    let mut token_map = parse("(begin (define r 10) (+ r r))".to_string(), &mut exp_map);
    let exp: ExprKind = read_from_tokens(&mut token_map).unwrap().into();

    let env = std_env();
    let mut symbol_definitions: HashMap<String, ExprKind> = HashMap::new();
    let res = eval(exp, &env, &mut symbol_definitions).unwrap();

    assert_eq!(
        res,
        ExprKind::Atom(Arc::new(Atom::Number(RLispNumber::Int(20))))
    );
}

#[test]
fn parse_and_eval_list_append() {
    setup_logging();

    let mut exp_map: HashMap<String, VecDeque<String>> = HashMap::new();
    let mut token_map = parse("(begin (append (1 2) 1))".to_string(), &mut exp_map);
    let exp: ExprKind = read_from_tokens(&mut token_map).unwrap().into();

    let env = std_env();
    let mut symbol_definitions: HashMap<String, ExprKind> = HashMap::new();
    let res = eval(exp, &env, &mut symbol_definitions).unwrap();

    assert_eq!(
        res,
        ExprKind::List(Arc::new(List {
            args: vec![
                ExprKind::Atom(Arc::new(Atom::Number(RLispNumber::Int(1)))),
                ExprKind::Atom(Arc::new(Atom::Number(RLispNumber::Int(2)))),
                ExprKind::Atom(Arc::new(Atom::Number(RLispNumber::Int(1)))),
            ],
            object_id: 0,
        }))
    );
}

#[test]
fn parse_and_eval_list_append_from_proc() {
    setup_logging();

    let mut exp_map: HashMap<String, VecDeque<String>> = HashMap::new();
    let mut token_map = parse("(begin (append (1 2) (+ 1 1)))".to_string(), &mut exp_map);
    let exp: ExprKind = read_from_tokens(&mut token_map).unwrap().into();

    let env = std_env();
    let mut symbol_definitions: HashMap<String, ExprKind> = HashMap::new();
    let res = eval(exp, &env, &mut symbol_definitions).unwrap();

    assert_eq!(
        res,
        ExprKind::List(Arc::new(List {
            args: vec![
                ExprKind::Atom(Arc::new(Atom::Number(RLispNumber::Int(1)))),
                ExprKind::Atom(Arc::new(Atom::Number(RLispNumber::Int(2)))),
                ExprKind::Atom(Arc::new(Atom::Number(RLispNumber::Int(2)))),
            ],
            object_id: 0,
        }))
    );
}

#[test]
fn parse_and_eval_list_append_and_print_proc() {
    setup_logging();

    let mut exp_map: HashMap<String, VecDeque<String>> = HashMap::new();
    let mut token_map = parse(
        "(begin (display (append (1 2) (+ 1 1))))".to_string(),
        &mut exp_map,
    );
    let exp: ExprKind = read_from_tokens(&mut token_map).unwrap().into();

    let env = std_env();
    let mut symbol_definitions: HashMap<String, ExprKind> = HashMap::new();
    let res = eval(exp, &env, &mut symbol_definitions).unwrap();

    assert_eq!(
        res,
        ExprKind::Atom(Arc::new(Atom::Number(RLispNumber::Int(0))))
    );
}

#[test]
fn parse_and_eval_lambda_define_and_invoke() {
    setup_logging();

    let mut exp_map: HashMap<String, VecDeque<String>> = HashMap::new();
    let mut token_map = parse(
        "(begin (define f (lambda (x) (* x x))) (f 4))".to_string(),
        &mut exp_map,
    );
    let exp: ExprKind = read_from_tokens(&mut token_map).unwrap().into();

    let env = std_env();
    let mut symbol_definitions: HashMap<String, ExprKind> = HashMap::new();
    let res = eval(exp, &env, &mut symbol_definitions).unwrap();

    assert_eq!(
        res,
        ExprKind::Atom(Arc::new(Atom::Number(RLispNumber::Int(16))))
    );
}

#[test]
fn parse_and_eval_recursive_factorial() {
    setup_logging();

    let mut exp_map: HashMap<String, VecDeque<String>> = HashMap::new();
    let mut token_map = parse(
        "(begin (define factorial (lambda (n) (if (< n 2) 1 (* n (factorial (- n 1)))))) (factorial 5))".to_string(),
        &mut exp_map,
    );
    let exp: ExprKind = read_from_tokens(&mut token_map).unwrap().into();

    let env = std_env();
    let mut symbol_definitions: HashMap<String, ExprKind> = HashMap::new();
    let res = eval(exp, &env, &mut symbol_definitions).unwrap();

    assert_eq!(
        res,
        ExprKind::Atom(Arc::new(Atom::Number(RLispNumber::Int(120))))
    );
}

#[test]
fn parse_and_eval_recursive_fibonacci() {
    setup_logging();

    let mut exp_map: HashMap<String, VecDeque<String>> = HashMap::new();
    let mut token_map = parse(
        "(begin (define fib (lambda (x) (if (< x 2) 1 (+ (fib (- x 1)) (fib (- x 2)))))) (fib 6))".to_string(),
        &mut exp_map,
    );
    let exp: ExprKind = read_from_tokens(&mut token_map).unwrap().into();

    let env = std_env();
    let mut symbol_definitions: HashMap<String, ExprKind> = HashMap::new();
    let res = eval(exp, &env, &mut symbol_definitions).unwrap();

    assert_eq!(
        res,
        ExprKind::Atom(Arc::new(Atom::Number(RLispNumber::Int(13))))
    );
}

#[test]
fn parse_and_eval_map_with_lambda() {
    setup_logging();

    let mut exp_map: HashMap<String, VecDeque<String>> = HashMap::new();
    let mut token_map = parse(
        "(begin (define a (map (lambda (x) (if (< 2 x) (+ x 1) (+ x 2))) '(1 2 3))) a)".to_string(),
        &mut exp_map,
    );
    let exp: ExprKind = read_from_tokens(&mut token_map).unwrap().into();

    let env = std_env();
    let mut symbol_definitions: HashMap<String, ExprKind> = HashMap::new();
    let res = eval(exp, &env, &mut symbol_definitions).unwrap();

    assert_eq!(
        res,
        ExprKind::Quote(Arc::new(Quote {
            expr: ExprKind::List(Arc::new(List {
                args: vec![
                    ExprKind::Atom(Arc::new(Atom::Number(RLispNumber::Int(3)))),
                    ExprKind::Atom(Arc::new(Atom::Number(RLispNumber::Int(4)))),
                    ExprKind::Atom(Arc::new(Atom::Number(RLispNumber::Int(4)))),
                ],
                object_id: 0,
            })),
        }))
    );
}

#[test]
fn parse_and_eval_condition() {
    setup_logging();

    let mut exp_map: HashMap<String, VecDeque<String>> = HashMap::new();
    let mut token_map = parse("(begin (if (number? 1) 1 2))".to_string(), &mut exp_map);
    let exp: ExprKind = read_from_tokens(&mut token_map).unwrap().into();

    let env = std_env();
    let mut symbol_definitions: HashMap<String, ExprKind> = HashMap::new();
    let res = eval(exp, &env, &mut symbol_definitions).unwrap();

    assert_eq!(
        res,
        ExprKind::Atom(Arc::new(Atom::Number(RLispNumber::Int(1))))
    );
}

#[test]
fn parse_and_eval_condition_as_var_definition() {
    setup_logging();

    let mut exp_map: HashMap<String, VecDeque<String>> = HashMap::new();
    let mut token_map = parse(
        "(begin (define f (if (number? 1) 1 2)) f)".to_string(),
        &mut exp_map,
    );
    let exp: ExprKind = read_from_tokens(&mut token_map).unwrap().into();

    let env = std_env();
    let mut symbol_definitions: HashMap<String, ExprKind> = HashMap::new();
    let res = eval(exp, &env, &mut symbol_definitions).unwrap();

    assert_eq!(
        res,
        ExprKind::Atom(Arc::new(Atom::Number(RLispNumber::Int(1))))
    );
}

#[test]
fn parse_and_eval_procedure_call_as_procedure_arg() {
    setup_logging();

    let mut exp_map: HashMap<String, VecDeque<String>> = HashMap::new();
    let mut token_map = parse(
        "(begin (if (number? (car (1 2 3))) (+ 2 2) 0))".to_string(),
        &mut exp_map,
    );
    let exp: ExprKind = read_from_tokens(&mut token_map).unwrap().into();

    let env = std_env();
    let mut symbol_definitions: HashMap<String, ExprKind> = HashMap::new();
    let res = eval(exp, &env, &mut symbol_definitions).unwrap();

    assert_eq!(
        res,
        ExprKind::Atom(Arc::new(Atom::Number(RLispNumber::Int(4))))
    );
}

#[test]
fn parse_and_eval_map_with_function_symbol() {
    setup_logging();

    let mut exp_map: HashMap<String, VecDeque<String>> = HashMap::new();
    let mut token_map = parse(
        "(begin (define foo (lambda (x) (+ 1 x))) (define f (map foo '(9))) f)".to_string(),
        &mut exp_map,
    );
    let exp: ExprKind = read_from_tokens(&mut token_map).unwrap().into();

    let env = std_env();
    let mut symbol_definitions: HashMap<String, ExprKind> = HashMap::new();
    let res = eval(exp, &env, &mut symbol_definitions).unwrap();

    assert_eq!(
        res,
        ExprKind::Quote(Arc::new(Quote {
            expr: ExprKind::List(Arc::new(List {
                args: vec![ExprKind::Atom(Arc::new(Atom::Number(RLispNumber::Int(10))))],
                object_id: 0,
            })),
        }))
    );
}

#[test]
fn parse_and_eval_filter_with_function() {
    setup_logging();

    let mut exp_map: HashMap<String, VecDeque<String>> = HashMap::new();
    let mut token_map = parse(
        "(begin
            (define b (filter (lambda (x) (
                if (< 2 x) #t #f))
            '(1 10 3)))

            b
        )".to_string().replace("\n", "").replace("\t", ""),
        &mut exp_map,
    );
    let exp: ExprKind = read_from_tokens(&mut token_map).unwrap().into();

    let env = std_env();
    let mut symbol_definitions: HashMap<String, ExprKind> = HashMap::new();
    let res = eval(exp, &env, &mut symbol_definitions).unwrap();

    assert_eq!(
        res,
        ExprKind::Quote(Arc::new(Quote{
            expr: ExprKind::List(Arc::new(List {
                args: vec![
                    ExprKind::Atom(Arc::new(Atom::Number(RLispNumber::Int(10)))),
                    ExprKind::Atom(Arc::new(Atom::Number(RLispNumber::Int(3)))),
                ],
                object_id: 0,
            })),
        }))
    );
}


#[test]
fn parse_and_eval_filter_with_function_symbol() {
    setup_logging();

    let mut exp_map: HashMap<String, VecDeque<String>> = HashMap::new();
    let mut token_map = parse(
        "(begin
            (define a '(1 10 3))
            (define b (filter (lambda (x) (
                if (< 2 x) #t #f))
            a))

            b
        )".to_string().replace("\n", "").replace("\t", ""),
        &mut exp_map,
    );
    let exp: ExprKind = read_from_tokens(&mut token_map).unwrap().into();

    let env = std_env();
    let mut symbol_definitions: HashMap<String, ExprKind> = HashMap::new();
    let res = eval(exp, &env, &mut symbol_definitions).unwrap();

    assert_eq!(
        res,
        ExprKind::Quote(Arc::new(Quote{
            expr: ExprKind::List(Arc::new(List {
                args: vec![
                    ExprKind::Atom(Arc::new(Atom::Number(RLispNumber::Int(10)))),
                    ExprKind::Atom(Arc::new(Atom::Number(RLispNumber::Int(3)))),
                ],
                object_id: 0,
            })),
        }))
    );
}


#[test]
fn parse_and_eval_eq_numerics() {
    setup_logging();

    let mut exp_map: HashMap<String, VecDeque<String>> = HashMap::new();
    let mut token_map = parse(
        "(begin
            (= 1 1)
        )".to_string().replace("\n", "").replace("\t", ""),
        &mut exp_map,
    );
    let exp: ExprKind = read_from_tokens(&mut token_map).unwrap().into();

    let env = std_env();
    let mut symbol_definitions: HashMap<String, ExprKind> = HashMap::new();
    let res = eval(exp, &env, &mut symbol_definitions).unwrap();

    assert_eq!(res, ExprKind::Atom(Arc::new(Atom::Bool(RLispBoolean::True(true)))));
}


#[test]
fn parse_and_eval_eq_booleans_true() {
    setup_logging();

    let mut exp_map: HashMap<String, VecDeque<String>> = HashMap::new();
    let mut token_map = parse(
        "(begin
            (= #f #f)
        )".to_string().replace("\n", "").replace("\t", ""),
        &mut exp_map,
    );
    let exp: ExprKind = read_from_tokens(&mut token_map).unwrap().into();

    let env = std_env();
    let mut symbol_definitions: HashMap<String, ExprKind> = HashMap::new();
    let res = eval(exp, &env, &mut symbol_definitions).unwrap();

    assert_eq!(res, ExprKind::Atom(Arc::new(Atom::Bool(RLispBoolean::True(true)))));
}

#[test]
fn parse_and_eval_eq_booleans_false() {
    setup_logging();

    let mut exp_map: HashMap<String, VecDeque<String>> = HashMap::new();
    let mut token_map = parse(
        "(begin
            (= #f #t)
        )".to_string().replace("\n", "").replace("\t", ""),
        &mut exp_map,
    );
    let exp: ExprKind = read_from_tokens(&mut token_map).unwrap().into();

    let env = std_env();
    let mut symbol_definitions: HashMap<String, ExprKind> = HashMap::new();
    let res = eval(exp, &env, &mut symbol_definitions).unwrap();

    assert_eq!(res, ExprKind::Atom(Arc::new(Atom::Bool(RLispBoolean::False(false)))));
}


#[test]
fn parse_and_eval_eq_lists_true() {
    setup_logging();

    let mut exp_map: HashMap<String, VecDeque<String>> = HashMap::new();
    let mut token_map = parse(
        "(begin
            (= (1 2) (1 2))
        )".to_string().replace("\n", "").replace("\t", ""),
        &mut exp_map,
    );
    let exp: ExprKind = read_from_tokens(&mut token_map).unwrap().into();

    let env = std_env();
    let mut symbol_definitions: HashMap<String, ExprKind> = HashMap::new();
    let res = eval(exp, &env, &mut symbol_definitions).unwrap();

    assert_eq!(res, ExprKind::Atom(Arc::new(Atom::Bool(RLispBoolean::True(true)))));
}


#[test]
fn parse_and_eval_eq_lists_false() {
    setup_logging();

    let mut exp_map: HashMap<String, VecDeque<String>> = HashMap::new();
    let mut token_map = parse(
        "(begin
            (= (1 2) (1 3))
        )".to_string().replace("\n", "").replace("\t", ""),
        &mut exp_map,
    );
    let exp: ExprKind = read_from_tokens(&mut token_map).unwrap().into();

    let env = std_env();
    let mut symbol_definitions: HashMap<String, ExprKind> = HashMap::new();
    let res = eval(exp, &env, &mut symbol_definitions).unwrap();

    assert_eq!(res, ExprKind::Atom(Arc::new(Atom::Bool(RLispBoolean::False(false)))));
}

#[test]
fn parse_and_eval_eq_lists_with_eval() {
    setup_logging();

    let mut exp_map: HashMap<String, VecDeque<String>> = HashMap::new();
    let mut token_map = parse(
        "(begin
            (= (1 2) (1 (+ 1 1)))
        )".to_string().replace("\n", "").replace("\t", ""),
        &mut exp_map,
    );
    let exp: ExprKind = read_from_tokens(&mut token_map).unwrap().into();

    let env = std_env();
    let mut symbol_definitions: HashMap<String, ExprKind> = HashMap::new();
    let res = eval(exp, &env, &mut symbol_definitions).unwrap();

    assert_eq!(res, ExprKind::Atom(Arc::new(Atom::Bool(RLispBoolean::True(true)))));
}



#[test]
fn parse_and_eval_let() {
    setup_logging();

    let mut exp_map: HashMap<String, VecDeque<String>> = HashMap::new();
    let mut token_map = parse(
        "(begin
            (define foo (1 2 3))
            (define adder (lambda (x y) (let
                    (
                        (define a (car foo))
                        (define b (car (cdr foo)))
                    )
                    (list a b x y)
                )
            ))

            (adder (1 2))
        )".to_string().replace("\n", "").replace("\t", ""),
        &mut exp_map,
    );
    let exp: ExprKind = read_from_tokens(&mut token_map).unwrap().into();

    let env = std_env();
    let mut symbol_definitions: HashMap<String, ExprKind> = HashMap::new();
    let res = eval(exp, &env, &mut symbol_definitions).unwrap();

    assert_eq!(res, ExprKind::Quote(Arc::new(Quote{
        expr: ExprKind::List(Arc::new(List {
            args: vec![
                ExprKind::Atom(Arc::new(Atom::Number(RLispNumber::Int(1)))),
                ExprKind::Atom(Arc::new(Atom::Number(RLispNumber::Int(2)))),
                ExprKind::Atom(Arc::new(Atom::Number(RLispNumber::Int(1)))),
                ExprKind::Atom(Arc::new(Atom::Number(RLispNumber::Int(2)))),
            ],
            object_id: 0,
        })),
    })));
}
