use std::collections::{HashMap, VecDeque};

use schemer::env::std_env;
use schemer::eval::eval;
use schemer::lisp;
use schemer::parser::{parse, read_from_tokens};
use schemer::types::SymbolicExpression;

fn setup_logging() {
    pretty_env_logger::try_init().unwrap_or(());
}

#[test]
fn basic_parse_and_eval() {
    setup_logging();

    let mut exp_map: HashMap<String, VecDeque<String>> = HashMap::new();
    let mut token_map = parse("(begin (+ 1 1))".to_string(), &mut exp_map);
    let exp = read_from_tokens(&mut token_map).unwrap();

    let env = std_env();
    let mut symbol_definitions: HashMap<String, SymbolicExpression> = HashMap::new();
    let res = eval(&exp, &env, &mut symbol_definitions).unwrap();

    assert_eq!(res, SymbolicExpression::Atom("2".to_string()))
}

#[test]
fn parse_and_eval_nested_operations() {
    setup_logging();

    let mut exp_map: HashMap<String, VecDeque<String>> = HashMap::new();
    let mut token_map = parse("(begin (+ (+ 1 1) (+ 1 1)))".to_string(), &mut exp_map);
    let exp = read_from_tokens(&mut token_map).unwrap();

    let env = std_env();
    let mut symbol_definitions: HashMap<String, SymbolicExpression> = HashMap::new();
    let res = eval(&exp, &env, &mut symbol_definitions).unwrap();

    assert_eq!(res, SymbolicExpression::Atom("4".to_string()))
}

#[test]
fn parse_and_eval_var_declare_and_resolve_for_proc() {
    setup_logging();

    let mut exp_map: HashMap<String, VecDeque<String>> = HashMap::new();
    let mut token_map = parse("(begin (define r 10) (+ r r))".to_string(), &mut exp_map);
    let exp = read_from_tokens(&mut token_map).unwrap();

    let env = std_env();
    let mut symbol_definitions: HashMap<String, SymbolicExpression> = HashMap::new();
    let res = eval(&exp, &env, &mut symbol_definitions).unwrap();

    //debug!("expressions: {:?}", exp);
    assert_eq!(res, SymbolicExpression::Atom("20".to_string()))
}

#[test]
fn parse_and_eval_list_append() {
    setup_logging();

    let mut exp_map: HashMap<String, VecDeque<String>> = HashMap::new();
    let mut token_map = parse("(begin (append (1 2) 1))".to_string(), &mut exp_map);
    let exp = read_from_tokens(&mut token_map).unwrap();

    let env = std_env();
    let mut symbol_definitions: HashMap<String, SymbolicExpression> = HashMap::new();
    let res = eval(&exp, &env, &mut symbol_definitions).unwrap();

    //debug!("expressions: {:?}", exp);
    assert_eq!(
        res,
        SymbolicExpression::List(vec![
            SymbolicExpression::Atom("1".to_string()),
            SymbolicExpression::Atom("2".to_string()),
            SymbolicExpression::Atom("1".to_string())
        ]),
    )
}

#[test]
fn parse_and_eval_list_append_from_proc() {
    setup_logging();

    let mut exp_map: HashMap<String, VecDeque<String>> = HashMap::new();
    let mut token_map = parse("(begin (append (1 2) (+ 1 1)))".to_string(), &mut exp_map);
    let exp = read_from_tokens(&mut token_map).unwrap();

    let env = std_env();
    let mut symbol_definitions: HashMap<String, SymbolicExpression> = HashMap::new();
    let res = eval(&exp, &env, &mut symbol_definitions).unwrap();

    //debug!("expressions: {:?}", exp);
    assert_eq!(
        res,
        SymbolicExpression::List(vec![
            SymbolicExpression::Atom("1".to_string()),
            SymbolicExpression::Atom("2".to_string()),
            SymbolicExpression::Atom("2".to_string())
        ])
    )
}

#[test]
fn parse_and_eval_list_append_and_print_proc() {
    setup_logging();

    let mut exp_map: HashMap<String, VecDeque<String>> = HashMap::new();
    let mut token_map = parse(
        "(begin (print (append (1 2) (+ 1 1))))".to_string(),
        &mut exp_map,
    );
    let exp = read_from_tokens(&mut token_map).unwrap();

    let env = std_env();
    let mut symbol_definitions: HashMap<String, SymbolicExpression> = HashMap::new();
    let res = eval(&exp, &env, &mut symbol_definitions).unwrap();

    //debug!("expressions: {:?}", exp);
    assert_eq!(res, SymbolicExpression::Atom("0".to_string()))
}

#[test]
fn parse_and_eval_lambda_define_and_invoke() {
    setup_logging();

    let mut exp_map: HashMap<String, VecDeque<String>> = HashMap::new();
    let mut token_map = parse(
        "(begin (define f (lambda (x) (* x x))) (f 4))".to_string(),
        &mut exp_map,
    );
    let exp = read_from_tokens(&mut token_map).unwrap();

    let env = std_env();
    let mut symbol_definitions: HashMap<String, SymbolicExpression> = HashMap::new();
    let res = eval(&exp, &env, &mut symbol_definitions).unwrap();

    assert_eq!(res, SymbolicExpression::Atom("16".to_string()))
}

#[test]
fn parse_and_eval_recursive_factorial() {
    setup_logging();

    let mut exp_map: HashMap<String, VecDeque<String>> = HashMap::new();
    let mut token_map = parse(
        "(begin (define factorial (lambda (n) (if (< n 2) 1 (* n (factorial (- n 1)))))) (factorial 5))".to_string(),
        &mut exp_map,
    );
    let exp = read_from_tokens(&mut token_map).unwrap();

    let env = std_env();
    let mut symbol_definitions: HashMap<String, SymbolicExpression> = HashMap::new();
    let res = eval(&exp, &env, &mut symbol_definitions).unwrap();

    assert_eq!(res, SymbolicExpression::Atom("120".to_string()))
}

#[test]
fn parse_and_eval_recursive_fibonacci() {
    setup_logging();

    let mut exp_map: HashMap<String, VecDeque<String>> = HashMap::new();
    let mut token_map = parse(
        "(begin (define fib (lambda (x) (if (< x 2) 1 (+ (fib (- x 1)) (fib (- x 2)))))) (fib 6))".to_string(),
        &mut exp_map,
    );
    let exp = read_from_tokens(&mut token_map).unwrap();

    let env = std_env();
    let mut symbol_definitions: HashMap<String, SymbolicExpression> = HashMap::new();
    let res = eval(&exp, &env, &mut symbol_definitions).unwrap();

    assert_eq!(res, SymbolicExpression::Atom("13".to_string()))
}

#[test]
fn parse_and_eval_map_with_lambda() {
    setup_logging();

    let mut exp_map: HashMap<String, VecDeque<String>> = HashMap::new();
    let mut token_map = parse(
        "(begin (define foo (lambda () (1 2 3))) (define a (map (lambda (x) (if (< 2 x) (+ x 1) (+ x 2))) (foo))) a)".to_string(),
        &mut exp_map,
    );
    let exp = read_from_tokens(&mut token_map).unwrap();

    let env = std_env();
    let mut symbol_definitions: HashMap<String, SymbolicExpression> = HashMap::new();
    let res = eval(&exp, &env, &mut symbol_definitions).unwrap();

    // foo returns (1 2 3), map applies lambda to each: 1+2=3, 2+2=4, 3+1=4
    assert_eq!(res, SymbolicExpression::List(vec![
        SymbolicExpression::Atom("3".to_string()),
        SymbolicExpression::Atom("4".to_string()),
        SymbolicExpression::Atom("4".to_string())
    ]))
}

#[test]
fn parse_and_eval_original_map_case() {
    setup_logging();

    let mut exp_map: HashMap<String, VecDeque<String>> = HashMap::new();
    let mut token_map = parse(
        "(begin (define foo (lambda () (1 2 3))) (define a (map (lambda (x) (if (< 2 x) (+ x 1) (+ x 2))) (foo))) a)".to_string(),
        &mut exp_map,
    );
    let exp = read_from_tokens(&mut token_map).unwrap();

    let env = std_env();
    let mut symbol_definitions: HashMap<String, SymbolicExpression> = HashMap::new();
    let res = eval(&exp, &env, &mut symbol_definitions).unwrap();

    // This should work without "invalid symbol x" error
    assert_eq!(res, SymbolicExpression::List(vec![
        SymbolicExpression::Atom("3".to_string()),
        SymbolicExpression::Atom("4".to_string()),
        SymbolicExpression::Atom("4".to_string())
    ]))
}

#[test]
fn parse_and_eval_map_with_function_calls() {
    setup_logging();

    let mut exp_map: HashMap<String, VecDeque<String>> = HashMap::new();
    let mut token_map = parse(
        "(begin (define fib (lambda (x) (if (< x 2) 1 (+ (fib (- x 1)) (fib (- x 2)))))) (define f (map (lambda (x) (begin (fib x))) (8 9 10))) f)".to_string(),
        &mut exp_map,
    );
    let exp = read_from_tokens(&mut token_map).unwrap();

    let env = std_env();
    let mut symbol_definitions: HashMap<String, SymbolicExpression> = HashMap::new();
    let res = eval(&exp, &env, &mut symbol_definitions).unwrap();

    // Should evaluate fib for each element: fib(1)=1, fib(2)=2, fib(3)=3
    assert_eq!(res, SymbolicExpression::List(vec![
        SymbolicExpression::Atom("34".to_string()),
        SymbolicExpression::Atom("55".to_string()),
        SymbolicExpression::Atom("89".to_string())
    ]))
}

#[test]
fn parse_and_eval_condition() {
    setup_logging();

    let mut exp_map: HashMap<String, VecDeque<String>> = HashMap::new();
    let mut token_map = parse("(begin (if (number? 1) 1 2))".to_string(), &mut exp_map);
    let exp = read_from_tokens(&mut token_map).unwrap();

    let env = std_env();
    let mut symbol_definitions: HashMap<String, SymbolicExpression> = HashMap::new();
    let res = eval(&exp, &env, &mut symbol_definitions).unwrap();

    //debug!("expressions: {:?}", exp);
    assert_eq!(res, SymbolicExpression::Atom("1".to_string()))
}

#[test]
fn parse_and_eval_condition_as_var_definition() {
    setup_logging();

    let mut exp_map: HashMap<String, VecDeque<String>> = HashMap::new();
    let mut token_map = parse(
        "(begin (define f (if (number? 1) 1 2)) f)".to_string(),
        &mut exp_map,
    );
    let exp = read_from_tokens(&mut token_map).unwrap();

    let env = std_env();
    let mut symbol_definitions: HashMap<String, SymbolicExpression> = HashMap::new();
    let res = eval(&exp, &env, &mut symbol_definitions).unwrap();

    //debug!("expressions: {:?}", exp);
    assert_eq!(res, SymbolicExpression::Atom("1".to_string()))
}

#[test]
fn parse_and_eval_procedure_call_as_procedure_arg() {
    setup_logging();

    let mut exp_map: HashMap<String, VecDeque<String>> = HashMap::new();
    let mut token_map = parse(
        "(begin (if (number? (car (1 2 3))) (+ 2 2) 0)) )".to_string(),
        &mut exp_map,
    );
    let exp = read_from_tokens(&mut token_map).unwrap();

    let env = std_env();
    let mut symbol_definitions: HashMap<String, SymbolicExpression> = HashMap::new();
    let res = eval(&exp, &env, &mut symbol_definitions).unwrap();

    //debug!("expressions: {:?}", exp);
    assert_eq!(res, SymbolicExpression::Atom("4".to_string()))
}
