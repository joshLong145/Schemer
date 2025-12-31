use schemer::lisp;
use schemer::types::SymbolicExpression;
use std::collections::HashMap;

fn setup_logging() {
    pretty_env_logger::try_init().unwrap_or(());
}

#[test]
fn basic_literal_parse() {
    setup_logging();

    let ast = lisp! {
        (1 1 1)
    };

    assert_eq!(
        ast,
        SymbolicExpression::List(vec![
            SymbolicExpression::Atom(String::from("1")),
            SymbolicExpression::Atom(String::from("1")),
            SymbolicExpression::Atom(String::from("1")),
        ])
    )
}

#[test]
fn basic_operator_call_parse() {
    setup_logging();

    let ast = lisp! {
        (+ 1 1)
    };

    assert_eq!(
        ast,
        SymbolicExpression::List(vec![
            SymbolicExpression::Atom(String::from("+")),
            SymbolicExpression::Atom(String::from("1")),
            SymbolicExpression::Atom(String::from("1")),
        ])
    )
}

#[test]
fn basic_operator_call_with_nesting_parse() {
    setup_logging();

    let ast = lisp! {
        (+ 1 (+ 1 1))
    };

    assert_eq!(
        ast,
        SymbolicExpression::List(vec![
            SymbolicExpression::Atom(String::from("+")),
            SymbolicExpression::Atom(String::from("1")),
            SymbolicExpression::List(vec![
                SymbolicExpression::Atom(String::from("+")),
                SymbolicExpression::Atom(String::from("1")),
                SymbolicExpression::Atom(String::from("1"))
            ]),
        ])
    )
}

#[test]
fn basic_operator_with_define() {
    setup_logging();

    let ast = lisp! {
        (define r 1 (+ r 1))
    };

    assert_eq!(
        ast,
        SymbolicExpression::List(vec![
            SymbolicExpression::Atom(String::from("define")),
            SymbolicExpression::Atom(String::from("r")),
            SymbolicExpression::Atom(String::from("1")),
            SymbolicExpression::List(vec![
                SymbolicExpression::Atom(String::from("+")),
                SymbolicExpression::Atom(String::from("r")),
                SymbolicExpression::Atom(String::from("1"))
            ]),
        ])
    )
}

#[test]
fn basic_operator_with_print() {
    setup_logging();

    let ast = lisp! {
        (define r 1 (print r))
    };

    assert_eq!(
        ast,
        SymbolicExpression::List(vec![
            SymbolicExpression::Atom(String::from("define")),
            SymbolicExpression::Atom(String::from("r")),
            SymbolicExpression::Atom(String::from("1")),
            SymbolicExpression::List(vec![
                SymbolicExpression::Atom(String::from("print")),
                SymbolicExpression::Atom(String::from("r")),
            ]),
        ])
    )
}

#[test]
fn basic_operator_with_proc() {
    setup_logging();

    let ast = lisp! {
        (define r 1 (cdr (1 2 3 r)))
    };

    assert_eq!(
        ast,
        SymbolicExpression::List(vec![
            SymbolicExpression::Atom(String::from("define")),
            SymbolicExpression::Atom(String::from("r")),
            SymbolicExpression::Atom(String::from("1")),
            SymbolicExpression::List(vec![
                SymbolicExpression::Atom(String::from("cdr")),
                SymbolicExpression::List(vec![
                    SymbolicExpression::Atom(String::from("1")),
                    SymbolicExpression::Atom(String::from("2")),
                    SymbolicExpression::Atom(String::from("3")),
                    SymbolicExpression::Atom(String::from("r")),
                ])
            ]),
        ])
    )
}
