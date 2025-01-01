use std::collections::{HashMap, VecDeque};

use schemer::env::std_env;
use schemer::eval::eval;
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
    assert_eq!(res, SymbolicExpression::Atom("0".to_string()),)
}
