use std::collections::VecDeque;
use std::sync::Arc;

use crate::{
    error::ParserError,
    types::value::Number,
    types::{SchemeList, Value},
};

/// Parse a token into an atomic Value (number, boolean, symbol)
fn parse_atom(token: &str) -> Value {
    // Try to parse as integer
    if let Ok(i) = token.parse::<i64>() {
        return Value::Number(Number::Int(i));
    }

    // Try to parse as float
    if let Ok(f) = token.parse::<f64>() {
        return Value::Number(Number::Float(f));
    }

    // Check for booleans
    if token == "#t" || token == "#true" {
        return Value::Boolean(true);
    }
    if token == "#f" || token == "#false" {
        return Value::Boolean(false);
    }

    // Otherwise it's a symbol
    Value::Symbol(token.to_string())
}

/// Tokenize and read a single Value from a program string
pub fn read(program: &str) -> Result<Value, ParserError> {
    let binding = program
        .replace("'", " ' ")
        .replace(")", " ) ")
        .replace("(", " ( ")
        .replace("\"", " \" ")
        .replace("#", " # ");
    let formatted_program: Vec<&str> = binding.split(' ').collect();

    let mut tokens: VecDeque<String> = formatted_program
        .iter()
        .map(|t| t.to_string())
        .filter(|t| !t.is_empty())
        .collect();

    read_value(&mut tokens)
}

/// Tokenize and read all Values from a program string (for multi-expression files)
pub fn read_all(program: &str) -> Result<Vec<Value>, ParserError> {
    let binding = program
        .replace("'", " ' ")
        .replace(")", " ) ")
        .replace("(", " ( ")
        .replace("\"", " \" ")
        .replace("#", " # ");

    let mut tokens: VecDeque<String> = binding.split_whitespace().map(|t| t.to_string()).collect();

    let mut expressions = Vec::new();
    while !tokens.is_empty() {
        expressions.push(read_value(&mut tokens)?);
    }
    Ok(expressions)
}

/// Read a Value from a VecDeque directly (public for use with pre-tokenized input)
pub fn read_value(tokens: &mut VecDeque<String>) -> Result<Value, ParserError> {
    if tokens.is_empty() {
        return Err(ParserError {
            msg: "unexpected end of input".to_string(),
        });
    }

    let token = tokens.pop_front().unwrap();

    if token == "(" {
        // Check for empty list
        if !tokens.is_empty() && tokens[0] == ")" {
            tokens.pop_front();
            return Ok(Value::Nil);
        }

        // Read list elements
        let mut elements: Vec<Value> = Vec::new();
        while !tokens.is_empty() && tokens[0] != ")" && tokens[0] != "." {
            let elem = read_value(tokens)?;
            elements.push(elem);
        }

        if tokens.is_empty() {
            return Err(ParserError {
                msg: "unexpected end of input, expected ')'".to_string(),
            });
        }

        // Check for dotted pair notation
        if tokens[0] == "." {
            tokens.pop_front(); // consume the dot
            let cdr = read_value(tokens)?;

            if tokens.is_empty() || tokens[0] != ")" {
                return Err(ParserError {
                    msg: "expected ')' after dotted pair".to_string(),
                });
            }
            tokens.pop_front(); // consume the closing paren

            // Build improper list from elements and final cdr
            // (a b . c) -> (a . (b . c))
            let mut result = cdr;
            for elem in elements.into_iter().rev() {
                result = Value::Pair(Arc::new((elem, result)));
            }
            return Ok(result);
        }

        // Consume closing paren
        if tokens[0] != ")" {
            return Err(ParserError {
                msg: format!("expected ')', got '{}'", tokens[0]),
            });
        }
        tokens.pop_front();

        // Build proper list using SchemeList
        let list = SchemeList::from_vec(elements);
        Ok(Value::List(Arc::new(list)))
    } else if token == ")" {
        Err(ParserError {
            msg: "unexpected ')'".to_string(),
        })
    } else if token == "'" {
        // Quote: 'x -> (quote x)
        let quoted = read_value(tokens)?;
        let quote_list = SchemeList::from_vec(vec![Value::Symbol("quote".to_string()), quoted]);
        Ok(Value::List(Arc::new(quote_list)))
    } else if token == "\"" {
        // String literal
        let mut s = String::new();
        while !tokens.is_empty() && tokens[0] != "\"" {
            let part = tokens.pop_front().unwrap();
            if !s.is_empty() {
                s.push(' ');
            }
            s.push_str(&part);
        }
        if tokens.is_empty() {
            return Err(ParserError {
                msg: "unterminated string literal".to_string(),
            });
        }
        tokens.pop_front(); // consume closing quote
        Ok(Value::String(Arc::new(s)))
    } else if token == "#" {
        // Character literal or boolean
        if tokens.is_empty() {
            return Err(ParserError {
                msg: "unexpected end of input after '#'".to_string(),
            });
        }
        let next = tokens.pop_front().unwrap();

        // Check for character literal: #\x or #\newline etc.
        if let Some(char_str) = next.strip_prefix('\\') {
            let c = match char_str {
                "newline" => '\n',
                "space" => ' ',
                "tab" => '\t',
                "return" => '\r',
                s if s.len() == 1 => s.chars().next().unwrap(),
                _ => {
                    return Err(ParserError {
                        msg: format!("invalid character literal: #\\{}", char_str),
                    });
                }
            };
            Ok(Value::Char(c))
        } else if next == "t" || next == "true" {
            Ok(Value::Boolean(true))
        } else if next == "f" || next == "false" {
            Ok(Value::Boolean(false))
        } else {
            // Unknown # syntax, treat as symbol
            Ok(Value::Symbol(format!("#{}", next)))
        }
    } else {
        // Regular atom (number, boolean, or symbol)
        Ok(parse_atom(&token))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::value::Number;

    #[test]
    fn test_read_integer() {
        let result = read("42").unwrap();
        assert_eq!(result, Value::Number(Number::Int(42)));
    }

    #[test]
    fn test_read_negative_integer() {
        let result = read("-123").unwrap();
        assert_eq!(result, Value::Number(Number::Int(-123)));
    }

    #[test]
    fn test_read_float() {
        let result = read("3.14").unwrap();
        assert_eq!(result, Value::Number(Number::Float(3.14)));
    }

    #[test]
    fn test_read_boolean_true() {
        let result = read("#t").unwrap();
        assert_eq!(result, Value::Boolean(true));
    }

    #[test]
    fn test_read_boolean_false() {
        let result = read("#f").unwrap();
        assert_eq!(result, Value::Boolean(false));
    }

    #[test]
    fn test_read_symbol() {
        let result = read("hello").unwrap();
        assert_eq!(result, Value::Symbol("hello".to_string()));
    }

    #[test]
    fn test_read_empty_list() {
        let result = read("()").unwrap();
        assert_eq!(result, Value::Nil);
    }

    #[test]
    fn test_read_simple_list() {
        let result = read("(1 2 3)").unwrap();
        match result {
            Value::List(list) => {
                assert_eq!(list.length(), 3);
                assert_eq!(list.nth(0), Some(Value::Number(Number::Int(1))));
                assert_eq!(list.nth(1), Some(Value::Number(Number::Int(2))));
                assert_eq!(list.nth(2), Some(Value::Number(Number::Int(3))));
            }
            _ => panic!("expected List, got {:?}", result),
        }
    }

    #[test]
    fn test_read_nested_list() {
        let result = read("(+ (- 5 3) 2)").unwrap();
        match result {
            Value::List(list) => {
                assert_eq!(list.length(), 3);
                assert_eq!(list.nth(0), Some(Value::Symbol("+".to_string())));
                // Second element is a nested list
                match list.nth(1) {
                    Some(Value::List(inner)) => {
                        assert_eq!(inner.length(), 3);
                        assert_eq!(inner.nth(0), Some(Value::Symbol("-".to_string())));
                    }
                    other => panic!("expected inner List, got {:?}", other),
                }
            }
            _ => panic!("expected List, got {:?}", result),
        }
    }

    #[test]
    fn test_read_quote() {
        let result = read("'x").unwrap();
        match result {
            Value::List(list) => {
                assert_eq!(list.length(), 2);
                assert_eq!(list.nth(0), Some(Value::Symbol("quote".to_string())));
                assert_eq!(list.nth(1), Some(Value::Symbol("x".to_string())));
            }
            _ => panic!("expected List (quote x), got {:?}", result),
        }
    }

    #[test]
    fn test_read_quoted_list() {
        let result = read("'(1 2 3)").unwrap();
        match result {
            Value::List(list) => {
                assert_eq!(list.length(), 2);
                assert_eq!(list.nth(0), Some(Value::Symbol("quote".to_string())));
                match list.nth(1) {
                    Some(Value::List(inner)) => {
                        assert_eq!(inner.length(), 3);
                    }
                    other => panic!("expected inner List, got {:?}", other),
                }
            }
            _ => panic!("expected quoted list, got {:?}", result),
        }
    }

    #[test]
    fn test_read_string() {
        let result = read("\"hello world\"").unwrap();
        match result {
            Value::String(s) => assert_eq!(*s, "hello world"),
            _ => panic!("expected String, got {:?}", result),
        }
    }

    #[test]
    fn test_read_character() {
        let result = read("#\\a").unwrap();
        assert_eq!(result, Value::Char('a'));
    }

    #[test]
    fn test_read_dotted_pair() {
        let result = read("(1 . 2)").unwrap();
        match result {
            Value::Pair(p) => {
                assert_eq!(p.0, Value::Number(Number::Int(1)));
                assert_eq!(p.1, Value::Number(Number::Int(2)));
            }
            _ => panic!("expected Pair, got {:?}", result),
        }
    }

    #[test]
    fn test_read_improper_list() {
        // (1 2 . 3) -> (1 . (2 . 3))
        let result = read("(1 2 . 3)").unwrap();
        match result {
            Value::Pair(p) => {
                assert_eq!(p.0, Value::Number(Number::Int(1)));
                match &p.1 {
                    Value::Pair(inner) => {
                        assert_eq!(inner.0, Value::Number(Number::Int(2)));
                        assert_eq!(inner.1, Value::Number(Number::Int(3)));
                    }
                    _ => panic!("expected inner Pair"),
                }
            }
            _ => panic!("expected Pair, got {:?}", result),
        }
    }

    #[test]
    fn test_read_define() {
        let result = read("(define x 42)").unwrap();
        match result {
            Value::List(list) => {
                assert_eq!(list.length(), 3);
                assert_eq!(list.nth(0), Some(Value::Symbol("define".to_string())));
                assert_eq!(list.nth(1), Some(Value::Symbol("x".to_string())));
                assert_eq!(list.nth(2), Some(Value::Number(Number::Int(42))));
            }
            _ => panic!("expected List, got {:?}", result),
        }
    }

    #[test]
    fn test_read_lambda() {
        let result = read("(lambda (x) (+ x 1))").unwrap();
        match result {
            Value::List(list) => {
                assert_eq!(list.length(), 3);
                assert_eq!(list.nth(0), Some(Value::Symbol("lambda".to_string())));
            }
            _ => panic!("expected List, got {:?}", result),
        }
    }
}
