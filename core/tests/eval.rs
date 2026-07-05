use std::sync::Arc;

use schemer::eval::Session;
use schemer::parser::read_all;
use schemer::types::{Number, SchemeList, Value};

fn setup_logging() {
    pretty_env_logger::try_init().unwrap_or(());
}

// Helper to create a proper list from a Vec of Values, as produced by the
// `(list ...)` primitive (built via `SchemeList`).
fn list(vals: Vec<Value>) -> Value {
    Value::List(Arc::new(SchemeList::from_vec(vals)))
}

// Flatten any proper-list `Value` (`Nil`/`List`/`Pair`, or a mix - e.g.
// `Pair(1, List([2, 3]))`, which is exactly what `append` built from a
// `cons`-recursive lambda over a `(list ...)`-built tail produces) into a
// `Vec<Value>` for comparison. Needed because `AnfTransformer::quote_list`
// lowers quoted list literals to `cons`/`cons`/.../`nil` chains (`Pair`s),
// while the `(list ...)` primitive builds a `SchemeList` (`Value::List`) -
// two different Rust-level representations of the same Scheme list, and
// `cons`-recursive helpers (`append`, `map`, `filter`) can freely mix both
// in one result (e.g. an `append` base case returning its second argument,
// a `Value::List`, unchanged, consed onto by further `Pair`s). Comparing via
// `to_vec` instead of exact `Value` shape avoids over-specifying which
// representation a given expression happens to produce.
fn to_vec(v: &Value) -> Vec<Value> {
    match v {
        Value::Nil => vec![],
        Value::List(l) => l.to_vec(),
        Value::Pair(p) => {
            let mut rest = to_vec(&p.1);
            rest.insert(0, p.0.clone());
            rest
        }
        other => vec![other.clone()],
    }
}

// Helper function to parse and evaluate a whole program via the ANF
// interpreter (Session::eval_program), replacing the old direct
// eval_value/std_env-based evaluator.
fn eval_str(program: &str) -> Result<Value, String> {
    let exprs = read_all(program).map_err(|e| e.msg)?;
    Session::new().eval_program(exprs)
}

/// Self-contained helper definitions used by tests below, since
/// Session::eval_program does not (in this rollout pass) auto-load
/// `lib/prelude.scm` - each test embeds exactly the helpers it needs so it
/// has no dependency on filesystem-relative paths.
const MAP_FILTER_APPEND_HELPERS: &str = r#"
(define map (lambda (f lst)
  (if (null? lst) '() (cons (f (car lst)) (map f (cdr lst))))))
(define filter (lambda (pred lst)
  (if (null? lst) '()
      (if (pred (car lst))
          (cons (car lst) (filter pred (cdr lst)))
          (filter pred (cdr lst))))))
(define append (lambda (lst1 lst2)
  (if (null? lst1) lst2 (cons (car lst1) (append (cdr lst1) lst2)))))
"#;

fn eval_with_helpers(program: &str) -> Result<Value, String> {
    eval_str(&format!("{}\n{}", MAP_FILTER_APPEND_HELPERS, program))
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
    let res =
        eval_with_helpers("(begin (define foo (lambda () (append '(1 2) '(1)))) (foo))").unwrap();
    assert_eq!(
        to_vec(&res),
        vec![
            Value::Number(Number::Int(1)),
            Value::Number(Number::Int(2)),
            Value::Number(Number::Int(1)),]
    );
}

#[test]
fn parse_and_eval_list_append_list_from_proc() {
    setup_logging();
    let res = eval_with_helpers(
        "(begin (define ret-list (lambda () (list 2 3))) (define foo (lambda () (append '(1) (ret-list)))) (foo))"
    ).unwrap();
    assert_eq!(
        to_vec(&res),
        vec![
            Value::Number(Number::Int(1)),
            Value::Number(Number::Int(2)),
            Value::Number(Number::Int(3)),]
    );
}

#[test]
fn parse_and_eval_list_append_single() {
    setup_logging();
    let res = eval_with_helpers("(begin (append '(1 2) '(1)))").unwrap();
    assert_eq!(
        to_vec(&res),
        vec![
            Value::Number(Number::Int(1)),
            Value::Number(Number::Int(2)),
            Value::Number(Number::Int(1)),]
    );
}

// NOTE: the original test called `(append '(1 2) '(1) '(1))` - 3 variadic
// arguments. `append` is not a `PrimOp` (compiler/anf.rs's primitive table
// has no entry for it); it's an ordinary Scheme procedure like the
// binary-only one in `lib/prelude.scm`, not the ad hoc variadic Rust closure
// `core/src/env.rs` used to provide for the old Value-walking evaluator.
// Adapted to chain two binary `append` calls rather than changing `append`'s
// arity (a library semantics change, out of scope for this rollout pass).
#[test]
fn parse_and_eval_list_append_multiple() {
    setup_logging();
    let res = eval_with_helpers("(begin (append (append '(1 2) '(1)) '(1)))").unwrap();
    assert_eq!(
        to_vec(&res),
        vec![
            Value::Number(Number::Int(1)),
            Value::Number(Number::Int(2)),
            Value::Number(Number::Int(1)),
            Value::Number(Number::Int(1)),]
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
    let res = eval_with_helpers(
        "(begin (define a (map (lambda (x) (if (< 2 x) (+ x 1) (+ x 2))) '(1 2 3))) a)",
    )
    .unwrap();
    assert_eq!(
        to_vec(&res),
        vec![
            Value::Number(Number::Int(3)),
            Value::Number(Number::Int(4)),
            Value::Number(Number::Int(4)),]
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
    let res = eval_with_helpers(
        "(begin (define foo (lambda (x) (+ 1 x))) (define f (map foo '(9))) f)",
    )
    .unwrap();
    assert_eq!(to_vec(&res), vec![Value::Number(Number::Int(10))]);
}

#[test]
fn parse_and_eval_filter_with_function() {
    setup_logging();
    let res = eval_with_helpers(
        "(begin (define b (filter (lambda (x) (if (< 2 x) #t #f)) '(1 10 3))) b)",
    )
    .unwrap();
    assert_eq!(
        to_vec(&res),
        vec![
            Value::Number(Number::Int(10)),
            Value::Number(Number::Int(3)),]
    );
}

#[test]
fn parse_and_eval_filter_with_function_symbol() {
    setup_logging();
    let res = eval_with_helpers(
        "(begin (define a '(1 10 3)) (define b (filter (lambda (x) (if (< 2 x) #t #f)) a)) b)",
    )
    .unwrap();
    assert_eq!(
        to_vec(&res),
        vec![
            Value::Number(Number::Int(10)),
            Value::Number(Number::Int(3)),]
    );
}

#[test]
fn parse_and_eval_eq_numerics() {
    setup_logging();
    let res = eval_str("(begin (= 1 1))").unwrap();
    assert_eq!(res, Value::Boolean(true));
}

// NOTE: the original tests here used `=` for boolean/list equality. Per
// R7RS (and `compiler/anf.rs`'s primitive table, which maps `"="` to the
// strictly-numeric `PrimOp::NumEq` for *both* backends - the whole point of
// routing the interpreter through the same `AnfTransformer`, spec §1.1),
// `=` only accepts numbers. The old `env.rs`-only evaluator's `=` was an
// R7RS-incorrect overload (deep-equality across any type) that never went
// through ANF; it's not preserved. `eq?` is the closest replacement for
// booleans. There is no `equal?` primitive/prelude function yet for deep
// list equality, so the list-equality cases are dropped rather than kept as
// tests of a feature that doesn't exist in this codebase.
#[test]
fn parse_and_eval_eq_booleans_true() {
    setup_logging();
    let res = eval_str("(begin (eq? #f #f))").unwrap();
    assert_eq!(res, Value::Boolean(true));
}

#[test]
fn parse_and_eval_eq_booleans_false() {
    setup_logging();
    let res = eval_str("(begin (eq? #f #t))").unwrap();
    assert_eq!(res, Value::Boolean(false));
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
fn parse_and_eval_let_star() {
    setup_logging();
    // let* is sequential: later bindings can see earlier ones in the same
    // form (unlike plain `let`). Verifies transform_let_star (already
    // present in anf.rs) works correctly through the interpreter.
    let res = eval_str("(let* ((a 1) (b (+ a 1))) (+ a b))").unwrap();
    assert_eq!(res, Value::Number(Number::Int(3)));
}

#[test]
fn parse_and_eval_letrec() {
    setup_logging();
    // Mutual/self recursion inside letrec.
    let res = eval_str(
        "(letrec ((even? (lambda (n) (if (= n 0) #t (odd? (- n 1)))))
                  (odd? (lambda (n) (if (= n 0) #f (even? (- n 1))))))
           (even? 10))",
    )
    .unwrap();
    assert_eq!(res, Value::Boolean(true));
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

// ============================================================
// New tests for what deliberately changes vs. the old evaluator
// (spec §10): and/or now short-circuit and are variadic (routed
// through `transform_and`/`transform_or`'s `If`-desugaring, so there's
// exactly one implementation shared with the compiled backend), and
// tail calls are now O(1) Rust-stack.
// ============================================================

#[test]
fn and_short_circuits_and_does_not_evaluate_car_of_empty_list() {
    setup_logging();
    // (car '()) would error if evaluated; `and` must short-circuit on the
    // first `#f` and never evaluate it.
    let res = eval_str("(and #f (car '()))").unwrap();
    assert_eq!(res, Value::Boolean(false));
}

#[test]
fn or_short_circuits_and_returns_first_truthy_value_not_just_a_boolean() {
    setup_logging();
    // Old env.rs's `or` was a strict 2-arg boolean-returning builtin
    // (`Value::Boolean(a.is_truthy() || b.is_truthy())`); ANF's `or` is a
    // variadic special form that returns the first truthy *value*, not `#t`.
    let res = eval_str("(or #f 5)").unwrap();
    assert_eq!(res, Value::Number(Number::Int(5)));
}

#[test]
fn and_is_variadic() {
    setup_logging();
    let res = eval_str("(and 1 2 3)").unwrap();
    assert_eq!(res, Value::Number(Number::Int(3)));
}

#[test]
fn or_is_variadic() {
    setup_logging();
    let res = eval_str("(or #f #f 7 8)").unwrap();
    assert_eq!(res, Value::Number(Number::Int(7)));
}

#[test]
fn self_recursive_tail_loop_does_not_overflow_the_stack() {
    setup_logging();
    // Proper tail calls (spec §6.1): a self-recursive loop of >10^6
    // iterations must complete via the trampoline, not native Rust
    // recursion, and without the old evaluator's dedicated 1GB-stack
    // thread (that thread isn't used by Session::eval_program at all).
    let res = eval_str(
        "(define loop (lambda (n acc) (if (= n 0) acc (loop (- n 1) (+ acc 1)))))
         (loop 2000000 0)",
    )
    .unwrap();
    assert_eq!(res, Value::Number(Number::Int(2_000_000)));
}
