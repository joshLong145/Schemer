/// Macro for constructing Scheme Value types at compile time
///
/// Example usage:
/// ```ignore
/// let val = lisp!((+ 1 2));  // Creates a list: (+ 1 2)
/// let num = lisp!(42);        // Creates a number: 42
/// ```
#[allow(unused_macros)]
#[macro_export]
macro_rules! lisp {
    // Escape hatch for Rust expressions
    ({$e:expr}) => {
       $e
    };

    // List syntax: (a b c)
    ( ( $($val:tt)* ) ) => {
        {
            let elements: Vec<$crate::types::Value> = vec![$(lisp!{$val}), *];
            if elements.is_empty() {
                $crate::types::Value::Nil
            } else {
                $crate::types::Value::List(std::sync::Arc::new(
                    $crate::types::SchemeList::from_vec(elements)
                ))
            }
        }
    };

    // Symbol: identifier
    ($x:ident) => {
        $crate::types::Value::Symbol(String::from(stringify!($x)))
    };

    // Operators as symbols
    (+) => {
        $crate::types::Value::Symbol(String::from("+"))
    };

    (-) => {
        $crate::types::Value::Symbol(String::from("-"))
    };

    (/) => {
        $crate::types::Value::Symbol(String::from("/"))
    };

    (*) => {
        $crate::types::Value::Symbol(String::from("*"))
    };

    // Integer literal
    ($exp:literal) => {
        $crate::parser::read(stringify!($exp)).unwrap()
    }
}
