use std::collections::{HashMap, VecDeque};
use std::sync::Arc;

use schemer::{
    env::{std_const_exp, std_env},
    eval::eval,
    parser::{parse, read_from_tokens},
    types::{ExprKind, Number, Value},
};

fn setup_logging() {
    pretty_env_logger::try_init().unwrap_or(());
}

// Helper to create a proper list from a Vec of Values
fn list(vals: Vec<Value>) -> Value {
    vals.into_iter()
        .rev()
        .fold(Value::Nil, |acc, val| Value::Pair(Arc::new((val, acc))))
}

#[test]
fn basic_parse_and_eval() {
    setup_logging();

    let mut exp_map: HashMap<String, VecDeque<String>> = HashMap::new();
    let mut token_map = parse("(begin (+ 1 1))".to_string(), &mut exp_map);
    let exp: ExprKind = read_from_tokens(&mut token_map).unwrap().into();

    let env = std_env();
    let mut symbol_definitions = std_const_exp();
    let res = eval(exp, &env, &mut symbol_definitions).unwrap();

    assert_eq!(res, Value::Number(Number::Int(2)));
}

#[test]
fn parse_and_eval_nested_operations() {
    setup_logging();

    let mut exp_map: HashMap<String, VecDeque<String>> = HashMap::new();
    let mut token_map = parse("(begin (+ (+ 1 1) (+ 1 1)))".to_string(), &mut exp_map);
    let exp: ExprKind = read_from_tokens(&mut token_map).unwrap().into();

    let env = std_env();
    let mut symbol_definitions = std_const_exp();
    let res = eval(exp, &env, &mut symbol_definitions).unwrap();

    assert_eq!(res, Value::Number(Number::Int(4)));
}

#[test]
fn parse_and_eval_var_declare_and_resolve_for_proc() {
    setup_logging();

    let mut exp_map: HashMap<String, VecDeque<String>> = HashMap::new();
    let mut token_map = parse("(begin (define r 10) (+ r r))".to_string(), &mut exp_map);
    let exp: ExprKind = read_from_tokens(&mut token_map).unwrap().into();

    let env = std_env();
    let mut symbol_definitions = std_const_exp();
    let res = eval(exp, &env, &mut symbol_definitions).unwrap();

    assert_eq!(res, Value::Number(Number::Int(20)));
}

#[test]
fn parse_and_eval_list_append_from_proc() {
    setup_logging();

    let mut exp_map: HashMap<String, VecDeque<String>> = HashMap::new();
    let mut token_map = parse(
        "(begin (define foo (lambda () (append '(1 2) '(1)))) (foo))".to_string(),
        &mut exp_map,
    );
    let exp: ExprKind = read_from_tokens(&mut token_map).unwrap().into();

    let env = std_env();
    let mut symbol_definitions = std_const_exp();
    let res = eval(exp, &env, &mut symbol_definitions).unwrap();

    assert_eq!(
        res,
        list(vec![
            Value::Number(Number::Int(1)),
            Value::Number(Number::Int(2)),
            Value::Number(Number::Int(1)),
        ])
    );
}

#[test]
fn parse_and_eval_list_append_list_from_proc() {
    setup_logging();

    let mut exp_map: HashMap<String, VecDeque<String>> = HashMap::new();
    let mut token_map = parse(
        "(begin (define ret-list (lambda () (list 2 3))) (define foo (lambda () (append '(1) (ret-list)))) (foo))"
            .to_string(),
        &mut exp_map,
    );
    let exp: ExprKind = read_from_tokens(&mut token_map).unwrap().into();

    let env = std_env();
    let mut symbol_definitions = std_const_exp();
    let res = eval(exp, &env, &mut symbol_definitions).unwrap();

    assert_eq!(
        res,
        list(vec![
            Value::Number(Number::Int(1)),
            Value::Number(Number::Int(2)),
            Value::Number(Number::Int(3)),
        ])
    );
}

#[test]
fn parse_and_eval_list_append_single() {
    setup_logging();

    let mut exp_map: HashMap<String, VecDeque<String>> = HashMap::new();
    let mut token_map = parse("(begin (append '(1 2) '(1)))".to_string(), &mut exp_map);
    let exp: ExprKind = read_from_tokens(&mut token_map).unwrap().into();

    let env = std_env();
    let mut symbol_definitions = std_const_exp();
    let res = eval(exp, &env, &mut symbol_definitions).unwrap();

    assert_eq!(
        res,
        list(vec![
            Value::Number(Number::Int(1)),
            Value::Number(Number::Int(2)),
            Value::Number(Number::Int(1)),
        ])
    );
}

#[test]
fn parse_and_eval_list_append_multiple() {
    setup_logging();

    let mut exp_map: HashMap<String, VecDeque<String>> = HashMap::new();
    let mut token_map = parse(
        "(begin (append '(1 2) '(1) '(1)))".to_string(),
        &mut exp_map,
    );
    let exp: ExprKind = read_from_tokens(&mut token_map).unwrap().into();

    let env = std_env();
    let mut symbol_definitions = std_const_exp();
    let res = eval(exp, &env, &mut symbol_definitions).unwrap();

    assert_eq!(
        res,
        list(vec![
            Value::Number(Number::Int(1)),
            Value::Number(Number::Int(2)),
            Value::Number(Number::Int(1)),
            Value::Number(Number::Int(1)),
        ])
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
    let mut symbol_definitions = std_const_exp();
    let res = eval(exp, &env, &mut symbol_definitions).unwrap();

    assert_eq!(res, Value::Number(Number::Int(16)));
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
    let mut symbol_definitions = std_const_exp();
    let res = eval(exp, &env, &mut symbol_definitions).unwrap();

    assert_eq!(res, Value::Number(Number::Int(120)));
}

#[test]
fn parse_and_eval_recursive_fibonacci() {
    setup_logging();

    let mut exp_map: HashMap<String, VecDeque<String>> = HashMap::new();
    let mut token_map = parse(
        "(begin (define fib (lambda (x) (if (< x 2) 1 (+ (fib (- x 1)) (fib (- x 2)))))) (fib 6))"
            .to_string(),
        &mut exp_map,
    );
    let exp: ExprKind = read_from_tokens(&mut token_map).unwrap().into();

    let env = std_env();
    let mut symbol_definitions = std_const_exp();
    let res = eval(exp, &env, &mut symbol_definitions).unwrap();

    assert_eq!(res, Value::Number(Number::Int(13)));
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
    let mut symbol_definitions = std_const_exp();
    let res = eval(exp, &env, &mut symbol_definitions).unwrap();

    assert_eq!(
        res,
        list(vec![
            Value::Number(Number::Int(3)),
            Value::Number(Number::Int(4)),
            Value::Number(Number::Int(4)),
        ])
    );
}

#[test]
fn parse_and_eval_condition() {
    setup_logging();

    let mut exp_map: HashMap<String, VecDeque<String>> = HashMap::new();
    let mut token_map = parse("(begin (if (number? 1) 1 2))".to_string(), &mut exp_map);
    let exp: ExprKind = read_from_tokens(&mut token_map).unwrap().into();

    let env = std_env();
    let mut symbol_definitions = std_const_exp();
    let res = eval(exp, &env, &mut symbol_definitions).unwrap();

    assert_eq!(res, Value::Number(Number::Int(1)));
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
    let mut symbol_definitions = std_const_exp();
    let res = eval(exp, &env, &mut symbol_definitions).unwrap();

    assert_eq!(res, Value::Number(Number::Int(1)));
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
    let mut symbol_definitions = std_const_exp();
    let res = eval(exp, &env, &mut symbol_definitions).unwrap();

    assert_eq!(res, Value::Number(Number::Int(4)));
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
    let mut symbol_definitions = std_const_exp();
    let res = eval(exp, &env, &mut symbol_definitions).unwrap();

    assert_eq!(res, list(vec![Value::Number(Number::Int(10))]));
}

#[test]
fn parse_and_eval_filter_with_function() {
    setup_logging();

    let mut exp_map: HashMap<String, VecDeque<String>> = HashMap::new();
    let mut token_map = parse(
        "(begin (define b (filter (lambda (x) (if (< 2 x) #t #f)) '(1 10 3))) b)".to_string(),
        &mut exp_map,
    );
    let exp: ExprKind = read_from_tokens(&mut token_map).unwrap().into();

    let env = std_env();
    let mut symbol_definitions = std_const_exp();
    let res = eval(exp, &env, &mut symbol_definitions).unwrap();

    assert_eq!(
        res,
        list(vec![
            Value::Number(Number::Int(10)),
            Value::Number(Number::Int(3)),
        ])
    );
}

#[test]
fn parse_and_eval_filter_with_function_symbol() {
    setup_logging();

    let mut exp_map: HashMap<String, VecDeque<String>> = HashMap::new();
    let mut token_map = parse(
        "(begin (define a '(1 10 3)) (define b (filter (lambda (x) (if (< 2 x) #t #f)) a)) b)"
            .to_string(),
        &mut exp_map,
    );
    let exp: ExprKind = read_from_tokens(&mut token_map).unwrap().into();

    let env = std_env();
    let mut symbol_definitions = std_const_exp();
    let res = eval(exp, &env, &mut symbol_definitions).unwrap();

    assert_eq!(
        res,
        list(vec![
            Value::Number(Number::Int(10)),
            Value::Number(Number::Int(3)),
        ])
    );
}

#[test]
fn parse_and_eval_eq_numerics() {
    setup_logging();

    let mut exp_map: HashMap<String, VecDeque<String>> = HashMap::new();
    let mut token_map = parse("(begin (= 1 1))".to_string(), &mut exp_map);
    let exp: ExprKind = read_from_tokens(&mut token_map).unwrap().into();

    let env = std_env();
    let mut symbol_definitions = std_const_exp();
    let res = eval(exp, &env, &mut symbol_definitions).unwrap();

    assert_eq!(res, Value::Boolean(true));
}

#[test]
fn parse_and_eval_eq_booleans_true() {
    setup_logging();

    let mut exp_map: HashMap<String, VecDeque<String>> = HashMap::new();
    let mut token_map = parse("(begin (= #f #f))".to_string(), &mut exp_map);
    let exp: ExprKind = read_from_tokens(&mut token_map).unwrap().into();

    let env = std_env();
    let mut symbol_definitions = std_const_exp();
    let res = eval(exp, &env, &mut symbol_definitions).unwrap();

    assert_eq!(res, Value::Boolean(true));
}

#[test]
fn parse_and_eval_eq_booleans_false() {
    setup_logging();

    let mut exp_map: HashMap<String, VecDeque<String>> = HashMap::new();
    let mut token_map = parse("(begin (= #f #t))".to_string(), &mut exp_map);
    let exp: ExprKind = read_from_tokens(&mut token_map).unwrap().into();

    let env = std_env();
    let mut symbol_definitions = std_const_exp();
    let res = eval(exp, &env, &mut symbol_definitions).unwrap();

    assert_eq!(res, Value::Boolean(false));
}

#[test]
fn parse_and_eval_eq_lists_true() {
    setup_logging();

    let mut exp_map: HashMap<String, VecDeque<String>> = HashMap::new();
    let mut token_map = parse("(begin (= (1 2) (1 2)))".to_string(), &mut exp_map);
    let exp: ExprKind = read_from_tokens(&mut token_map).unwrap().into();

    let env = std_env();
    let mut symbol_definitions = std_const_exp();
    let res = eval(exp, &env, &mut symbol_definitions).unwrap();

    assert_eq!(res, Value::Boolean(true));
}

#[test]
fn parse_and_eval_eq_lists_false() {
    setup_logging();

    let mut exp_map: HashMap<String, VecDeque<String>> = HashMap::new();
    let mut token_map = parse("(begin (= (1 2) (1 3)))".to_string(), &mut exp_map);
    let exp: ExprKind = read_from_tokens(&mut token_map).unwrap().into();

    let env = std_env();
    let mut symbol_definitions = std_const_exp();
    let res = eval(exp, &env, &mut symbol_definitions).unwrap();

    assert_eq!(res, Value::Boolean(false));
}

#[test]
fn parse_and_eval_eq_lists_with_eval() {
    setup_logging();

    let mut exp_map: HashMap<String, VecDeque<String>> = HashMap::new();
    let mut token_map = parse("(begin (= (1 2) (1 (+ 1 1))))".to_string(), &mut exp_map);
    let exp: ExprKind = read_from_tokens(&mut token_map).unwrap().into();

    let env = std_env();
    let mut symbol_definitions = std_const_exp();
    let res = eval(exp, &env, &mut symbol_definitions).unwrap();

    assert_eq!(res, Value::Boolean(true));
}

#[test]
fn parse_and_eval_let() {
    setup_logging();

    let mut exp_map: HashMap<String, VecDeque<String>> = HashMap::new();
    let mut token_map = parse(
        "(begin (define foo (1 2 3)) (define adder (lambda (x y) (let ((a (car foo)) (b (car (cdr foo)))) (list a b x y)))) (adder 1 2))"
            .to_string(),
        &mut exp_map,
    );
    let exp: ExprKind = read_from_tokens(&mut token_map).unwrap().into();

    let env = std_env();
    let mut symbol_definitions = std_const_exp();
    let res = eval(exp, &env, &mut symbol_definitions).unwrap();

    assert_eq!(
        res,
        list(vec![
            Value::Number(Number::Int(1)),
            Value::Number(Number::Int(2)),
            Value::Number(Number::Int(1)),
            Value::Number(Number::Int(2)),
        ])
    );
}

#[test]
fn parse_and_eval_char_define() {
    setup_logging();

    let mut exp_map: HashMap<String, VecDeque<String>> = HashMap::new();
    let mut token_map = parse("(begin (define a #\\a) a)".to_string(), &mut exp_map);
    let exp: ExprKind = read_from_tokens(&mut token_map).unwrap().into();

    let env = std_env();
    let mut symbol_definitions = std_const_exp();
    let res = eval(exp, &env, &mut symbol_definitions).unwrap();

    assert_eq!(res, Value::String(Arc::new("a".to_string())));
}

// R7RS truthiness tests
#[test]
fn test_r7rs_truthiness_only_false_is_falsy() {
    setup_logging();

    let mut exp_map: HashMap<String, VecDeque<String>> = HashMap::new();

    // 0 is truthy in R7RS
    let mut token_map = parse("(if 0 1 2)".to_string(), &mut exp_map);
    let exp: ExprKind = read_from_tokens(&mut token_map).unwrap().into();
    let env = std_env();
    let mut symbol_definitions = std_const_exp();
    let res = eval(exp, &env, &mut symbol_definitions).unwrap();
    assert_eq!(res, Value::Number(Number::Int(1)));

    // empty list is truthy in R7RS
    exp_map.clear();
    let mut token_map = parse("(if '() 1 2)".to_string(), &mut exp_map);
    let exp: ExprKind = read_from_tokens(&mut token_map).unwrap().into();
    let res = eval(exp, &env, &mut symbol_definitions).unwrap();
    assert_eq!(res, Value::Number(Number::Int(1)));

    // only #f is falsy
    exp_map.clear();
    let mut token_map = parse("(if #f 1 2)".to_string(), &mut exp_map);
    let exp: ExprKind = read_from_tokens(&mut token_map).unwrap().into();
    let res = eval(exp, &env, &mut symbol_definitions).unwrap();
    assert_eq!(res, Value::Number(Number::Int(2)));
}

#[test]
fn test_type_predicates() {
    setup_logging();
    let env = std_env();
    let mut symbol_definitions = std_const_exp();
    let mut exp_map: HashMap<String, VecDeque<String>> = HashMap::new();

    // number?
    let mut token_map = parse("(number? 42)".to_string(), &mut exp_map);
    let exp: ExprKind = read_from_tokens(&mut token_map).unwrap().into();
    assert_eq!(
        eval(exp, &env, &mut symbol_definitions).unwrap(),
        Value::Boolean(true)
    );

    // boolean?
    exp_map.clear();
    let mut token_map = parse("(boolean? #t)".to_string(), &mut exp_map);
    let exp: ExprKind = read_from_tokens(&mut token_map).unwrap().into();
    assert_eq!(
        eval(exp, &env, &mut symbol_definitions).unwrap(),
        Value::Boolean(true)
    );

    // null?
    exp_map.clear();
    let mut token_map = parse("(null? '())".to_string(), &mut exp_map);
    let exp: ExprKind = read_from_tokens(&mut token_map).unwrap().into();
    assert_eq!(
        eval(exp, &env, &mut symbol_definitions).unwrap(),
        Value::Boolean(true)
    );

    // pair?
    exp_map.clear();
    let mut token_map = parse("(pair? (cons 1 2))".to_string(), &mut exp_map);
    let exp: ExprKind = read_from_tokens(&mut token_map).unwrap().into();
    assert_eq!(
        eval(exp, &env, &mut symbol_definitions).unwrap(),
        Value::Boolean(true)
    );
}
