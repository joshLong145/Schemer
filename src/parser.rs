use std::collections::{HashMap, VecDeque};

use crate::{
    error::ParserError,
    types::{SymbolicExpression, Tokens},
};

pub fn parse<'a>(
    program: String,
    token_store: &'a mut HashMap<String, VecDeque<String>>,
) -> Tokens<'a> {
    let binding = program.replace(")", " ) ").replace("(", " ( ");
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
        let mut l: Vec<SymbolicExpression> = Vec::new();
        while tokens[0] != ")" {
            let token = read_from_tokens(tokens)?;
            l.push(token);
        }
        let _ = tokens.pop_front().unwrap();
        return Ok(SymbolicExpression::List(l));
    } else if token == ")" {
        return Err(ParserError {
            msg: "unexpected token".to_string(),
        });
    }

    Ok(SymbolicExpression::Atom(token))
}
