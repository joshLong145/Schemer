#[allow(unused_macros)]
#[macro_export]
macro_rules! lisp {
    ({$e:expr}) => {
       $e
    };

    ( ( $($val:tt)* ) ) => {
        $crate::types::SymbolicExpression::List([$(lisp!{$val}), *].iter().map(|v| v.to_owned()).collect::<Vec<SymbolicExpression>>())
    };

    ($x:ident) => {
        $crate::types::SymbolicExpression::Atom(String::from(stringify!($x)))
    };

    (+) => {
        $crate::types::SymbolicExpression::Atom(String::from("+"))
    };

    (-) => {
        $crate::types::SymbolicExpression::Atom(String::from("+"))
    };

    (/) => {
        $crate::types::SymbolicExpression::Atom(String::from("+"))
    };

    (*) => {
        $crate::types::SymbolicExpression::Atom(String::from("+"))
    };

    ($exp:literal) => {
        $crate::parser::read_from_tokens(&mut $crate::parser::parse(stringify!($exp).to_string(), &mut HashMap::new())).unwrap()
    }
}
