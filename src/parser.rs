use std::collections::{HashMap, VecDeque};

use crate::{
    error::ParserError,
    types::{SymbolicExpression, Tokens},
};

pub fn parse<'a>(
    program: String,
    token_store: &'a mut HashMap<String, VecDeque<String>>,
) -> Tokens<'a> {
    let binding = program
        .replace("'", " ' ")
        .replace(")", " ) ")
        .replace("(", " ( ")
        .replace("\"", " \" ")
        .replace("#", " # ");
    let formatted_program: Vec<&str> = binding.split(' ').collect();

    let converted_program: VecDeque<String> = formatted_program
        .iter()
        .map(|t| t.to_string())
        .filter(|t| t != "")
        .collect();

    token_store.insert(program.clone(), converted_program);

    token_store.get_mut(&program.clone()).unwrap()
}

pub fn read_from_tokens(tokens: &mut Tokens) -> Result<SymbolicExpression, ParserError> {
    let token = tokens.pop_front().unwrap();
    if token == "(" {
        let mut v: Vec<SymbolicExpression> = Vec::new();
        while tokens[0] != ")" {
            let token = read_from_tokens(tokens)?;
            v.push(token);
        }
        let _ = tokens.pop_front().unwrap();
        let l = SymbolicExpression::List(v.clone());
        if v.len() < 1 {
            return Ok(l);
        }

        if let Some(exp) = l.try_peek() {
            match exp {
                SymbolicExpression::Atom(a) => {
                    if a == "lambda" {
                        return Ok(SymbolicExpression::Lambda(v));
                    }

                    return Ok(l);
                }
                _ => {}
            }

            return Ok(l);
        }
    } else if token == ")" {
        return Err(ParserError {
            msg: "unexpected token".to_string(),
        });
    } else if token == "'" {
        let next_token = tokens.pop_front().unwrap();
        if next_token == "(" {
            let mut v: Vec<SymbolicExpression> = Vec::new();
            while tokens[0] != ")" {
                let token = read_from_tokens(tokens)?;
                v.push(token);
            }
            let _ = tokens.pop_front().unwrap();
            let l = SymbolicExpression::ListExpr(v.clone());
            return Ok(l);
        }
    } else if token == "\"" {
        let mut v: String = "".to_string();
        while tokens[0] != "\"" {
            let token = read_from_tokens(tokens)?;
            v = format!("{}{}", v, token).to_string();
        }
        let _ = tokens.pop_front().unwrap();
        let l = SymbolicExpression::StringLiteral(v.clone());

        return Ok(l);
    } else if token == "#" {
        let potential_chars: String = tokens.pop_front().unwrap();
        // take the second character from the iter, the first is a slash delim
        if let Some(char) = potential_chars.chars().nth(1) {
            return Ok(SymbolicExpression::Character(char));
        } else {
            return Ok(SymbolicExpression::Atom(format!("#{}", potential_chars)));
        }
    }

    Ok(SymbolicExpression::Atom(token))
}
