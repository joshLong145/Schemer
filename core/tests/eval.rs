use std::sync::Arc;

use schemer::{
    env::{std_const_exp, std_env},
    eval::eval_value,
    parser::read,
    types::{Number, SchemeList, Value},
};

fn setup_logging() {
    pretty_env_logger::try_init().unwrap_or(());
}

// Helper to create a proper list from a Vec of Values
fn list(vals: Vec<Value>) -> Value {
    Value::List(Arc::new(SchemeList::from_vec(vals)))
}

// Helper function to parse and evaluate
fn eval_str(program: &str) -> Result<Value, String> {
    let expr = read(program).map_err(|e| e.msg)?;
    let env = std_env();
    let mut defs = std_const_exp();
    eval_value(expr, &env, &mut defs)
}

#[test]
fn basic_parse_and_eval() {
    setup_logging();
    let res = eval_str("(begin (+ 1 1))").unwrap();
    assert_eq!(res, Value::Number(Number::Int(2)));
}

#[test]
fn parse_and_eval_nested_operations() {
    setup_logging();
    let res = eval_str("(begin (+ (+ 1 1) (+ 1 1)))").unwrap();
    assert_eq!(res, Value::Number(Number::Int(4)));
}

#[test]
fn parse_and_eval_var_declare_and_resolve_for_proc() {
    setup_logging();
    let res = eval_str("(begin (define r 10) (+ r r))").unwrap();
    assert_eq!(res, Value::Number(Number::Int(20)));
}

#[test]
fn parse_and_eval_list_append_from_proc() {
    setup_logging();
    let res = eval_str("(begin (define foo (lambda () (append '(1 2) '(1)))) (foo))").unwrap();
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
    let res = eval_str(
        "(begin (define ret-list (lambda () (list 2 3))) (define foo (lambda () (append '(1) (ret-list)))) (foo))"
    ).unwrap();
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
    let res = eval_str("(begin (append '(1 2) '(1)))").unwrap();
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
    let res = eval_str("(begin (append '(1 2) '(1) '(1)))").unwrap();
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
    let res = eval_str("(begin (define f (lambda (x) (* x x))) (f 4))").unwrap();
    assert_eq!(res, Value::Number(Number::Int(16)));
}

#[test]
fn parse_and_eval_recursive_factorial() {
    setup_logging();
    let res = eval_str(
        "(begin (define factorial (lambda (n) (if (< n 2) 1 (* n (factorial (- n 1)))))) (factorial 5))"
    ).unwrap();
    assert_eq!(res, Value::Number(Number::Int(120)));
}

#[test]
fn parse_and_eval_recursive_fibonacci() {
    setup_logging();
    let res = eval_str(
        "(begin (define fib (lambda (x) (if (< x 2) 1 (+ (fib (- x 1)) (fib (- x 2)))))) (fib 6))",
    )
    .unwrap();
    assert_eq!(res, Value::Number(Number::Int(13)));
}

#[test]
fn parse_and_eval_map_with_lambda() {
    setup_logging();
    let res =
        eval_str("(begin (define a (map (lambda (x) (if (< 2 x) (+ x 1) (+ x 2))) '(1 2 3))) a)")
            .unwrap();
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
    let res = eval_str("(begin (if (number? 1) 1 2))").unwrap();
    assert_eq!(res, Value::Number(Number::Int(1)));
}

#[test]
fn parse_and_eval_condition_as_var_definition() {
    setup_logging();
    let res = eval_str("(begin (define f (if (number? 1) 1 2)) f)").unwrap();
    assert_eq!(res, Value::Number(Number::Int(1)));
}

#[test]
fn parse_and_eval_procedure_call_as_procedure_arg() {
    setup_logging();
    let res = eval_str("(begin (if (number? (car '(1 2 3))) (+ 2 2) 0))").unwrap();
    assert_eq!(res, Value::Number(Number::Int(4)));
}

#[test]
fn parse_and_eval_map_with_function_symbol() {
    setup_logging();
    let res =
        eval_str("(begin (define foo (lambda (x) (+ 1 x))) (define f (map foo '(9))) f)").unwrap();
    assert_eq!(res, list(vec![Value::Number(Number::Int(10))]));
}

#[test]
fn parse_and_eval_filter_with_function() {
    setup_logging();
    let res = eval_str("(begin (define b (filter (lambda (x) (if (< 2 x) #t #f)) '(1 10 3))) b)")
        .unwrap();
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
    let res = eval_str(
        "(begin (define a '(1 10 3)) (define b (filter (lambda (x) (if (< 2 x) #t #f)) a)) b)",
    )
    .unwrap();
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
    let res = eval_str("(begin (= 1 1))").unwrap();
    assert_eq!(res, Value::Boolean(true));
}

#[test]
fn parse_and_eval_eq_booleans_true() {
    setup_logging();
    let res = eval_str("(begin (= #f #f))").unwrap();
    assert_eq!(res, Value::Boolean(true));
}

#[test]
fn parse_and_eval_eq_booleans_false() {
    setup_logging();
    let res = eval_str("(begin (= #f #t))").unwrap();
    assert_eq!(res, Value::Boolean(false));
}

#[test]
fn parse_and_eval_eq_lists_true() {
    setup_logging();
    let res = eval_str("(begin (= '(1 2) '(1 2)))").unwrap();
    assert_eq!(res, Value::Boolean(true));
}

#[test]
fn parse_and_eval_eq_lists_false() {
    setup_logging();
    let res = eval_str("(begin (= '(1 2) '(1 3)))").unwrap();
    assert_eq!(res, Value::Boolean(false));
}

#[test]
fn parse_and_eval_eq_lists_with_eval() {
    setup_logging();
    let res = eval_str("(begin (= '(1 2) (list 1 (+ 1 1))))").unwrap();
    assert_eq!(res, Value::Boolean(true));
}

#[test]
fn parse_and_eval_let() {
    setup_logging();
    let res = eval_str(
        "(begin (define foo '(1 2 3)) (define adder (lambda (x y) (let ((a (car foo)) (b (car (cdr foo)))) (list a b x y)))) (adder 1 2))"
    ).unwrap();
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
    let res = eval_str("(begin (define a #\\a) a)").unwrap();
    assert_eq!(res, Value::Char('a'));
}

// R7RS truthiness tests
#[test]
fn test_r7rs_truthiness_only_false_is_falsy() {
    setup_logging();

    // 0 is truthy in R7RS
    let res = eval_str("(if 0 1 2)").unwrap();
    assert_eq!(res, Value::Number(Number::Int(1)));

    // empty list is truthy in R7RS
    let res = eval_str("(if '() 1 2)").unwrap();
    assert_eq!(res, Value::Number(Number::Int(1)));

    // only #f is falsy
    let res = eval_str("(if #f 1 2)").unwrap();
    assert_eq!(res, Value::Number(Number::Int(2)));
}

#[test]
fn test_type_predicates() {
    setup_logging();

    // number?
    let res = eval_str("(number? 42)").unwrap();
    assert_eq!(res, Value::Boolean(true));

    // boolean?
    let res = eval_str("(boolean? #t)").unwrap();
    assert_eq!(res, Value::Boolean(true));

    // null?
    let res = eval_str("(null? '())").unwrap();
    assert_eq!(res, Value::Boolean(true));

    // pair?
    let res = eval_str("(pair? (cons 1 2))").unwrap();
    assert_eq!(res, Value::Boolean(true));
}
