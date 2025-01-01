
use rustyline::error::ReadlineError;
use rustyline::DefaultEditor;
use log::debug;
use schemer::env::std_env;
use schemer::eval::eval;
use schemer::parser::{parse, read_from_tokens};
use schemer::types::SymbolicExpression;
use std::collections::{HashMap, VecDeque};

pub fn repl() -> rustyline::Result<()> {
    let env = std_env();
    let mut exp_map: HashMap<String, VecDeque<String>> = HashMap::new();
    let mut symbol_definitions: HashMap<String, SymbolicExpression> = HashMap::new();

    let mut rl = DefaultEditor::new()?;

    loop {
        let readline = rl.readline(">> ");
        match readline {
            Ok(buffer) => {
                let _ = rl.add_history_entry(buffer.as_str());

                let mut token_map = parse(buffer, &mut exp_map);
                let exp = read_from_tokens(&mut token_map).unwrap();
                let res = eval(&exp, &env, &mut symbol_definitions).unwrap();
                println!("{}", res);
            }
            Err(ReadlineError::Interrupted) => {
                debug!("CTRL-C");
                break;
            }
            Err(ReadlineError::Eof) => {
                debug!("CTRL-D");
                break;
            }
            Err(err) => {
                debug!("Error: {:?}", err);
                break;
            }
        }
    }
    Ok(())
}

pub fn parse_and_run_scheme(buffer: String) {
    let env = std_env();
    let mut exp_map: HashMap<String, VecDeque<String>> = HashMap::new();
    let mut symbol_definitions: HashMap<String, SymbolicExpression> = HashMap::new();

    let mut token_map = parse(buffer, &mut exp_map);
    let exp = read_from_tokens(&mut token_map).unwrap();
    let res = eval(&exp, &env, &mut symbol_definitions).unwrap();

    println!("{}", res);
}