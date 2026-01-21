use schemer::lisp;
use schemer::types::Value;

fn setup_logging() {
    pretty_env_logger::try_init().unwrap_or(());
}

#[test]
fn basic_literal_parse() {
    setup_logging();

    let ast = lisp! {
        (1 1 1)
    };

    match ast {
        Value::List(list) => {
            assert_eq!(list.length(), 3);
        }
        _ => panic!("expected List"),
    }
}

#[test]
fn basic_operator_call_parse() {
    setup_logging();

    let ast = lisp! {
        (+ 1 1)
    };

    match ast {
        Value::List(list) => {
            assert_eq!(list.length(), 3);
            assert_eq!(list.nth(0), Some(Value::Symbol("+".to_string())));
        }
        _ => panic!("expected List"),
    }
}

#[test]
fn basic_operator_call_with_nesting_parse() {
    setup_logging();

    let ast = lisp! {
        (+ 1 (+ 1 1))
    };

    match ast {
        Value::List(list) => {
            assert_eq!(list.length(), 3);
            assert_eq!(list.nth(0), Some(Value::Symbol("+".to_string())));
            // Third element should be a nested list
            match list.nth(2) {
                Some(Value::List(inner)) => {
                    assert_eq!(inner.length(), 3);
                    assert_eq!(inner.nth(0), Some(Value::Symbol("+".to_string())));
                }
                other => panic!("expected nested List, got {:?}", other),
            }
        }
        _ => panic!("expected List"),
    }
}

#[test]
fn basic_operator_with_define() {
    setup_logging();

    let ast = lisp! {
        (define r 1 (+ r 1))
    };

    match ast {
        Value::List(list) => {
            assert_eq!(list.length(), 4);
            assert_eq!(list.nth(0), Some(Value::Symbol("define".to_string())));
            assert_eq!(list.nth(1), Some(Value::Symbol("r".to_string())));
        }
        _ => panic!("expected List"),
    }
}

#[test]
fn basic_operator_with_print() {
    setup_logging();

    let ast = lisp! {
        (define r 1 (print r))
    };

    match ast {
        Value::List(list) => {
            assert_eq!(list.length(), 4);
            assert_eq!(list.nth(0), Some(Value::Symbol("define".to_string())));
            match list.nth(3) {
                Some(Value::List(inner)) => {
                    assert_eq!(inner.nth(0), Some(Value::Symbol("print".to_string())));
                }
                other => panic!("expected nested List for print, got {:?}", other),
            }
        }
        _ => panic!("expected List"),
    }
}

#[test]
fn basic_operator_with_proc() {
    setup_logging();

    let ast = lisp! {
        (define r 1 (cdr (1 2 3 r)))
    };

    match ast {
        Value::List(list) => {
            assert_eq!(list.length(), 4);
            assert_eq!(list.nth(0), Some(Value::Symbol("define".to_string())));
            match list.nth(3) {
                Some(Value::List(inner)) => {
                    assert_eq!(inner.nth(0), Some(Value::Symbol("cdr".to_string())));
                }
                other => panic!("expected nested List for cdr, got {:?}", other),
            }
        }
        _ => panic!("expected List"),
    }
}
