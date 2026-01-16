use rustyline::error::ReadlineError;
use rustyline::DefaultEditor;
use schemer::env::std_env;
use schemer::eval::eval;
use schemer::parser::{parse, read_from_tokens};
use schemer::types::ExprKind;

use std::collections::{HashMap, VecDeque};
use std::thread;

pub fn repl() -> rustyline::Result<()> {
    let env = std_env();
    let mut exp_map: HashMap<String, VecDeque<String>> = HashMap::new();
    let mut symbol_definitions: HashMap<String, ExprKind> = HashMap::new();

    let mut rl = DefaultEditor::new()?;

    loop {
        let readline = rl.readline("ƛ > ");
        match readline {
            Ok(buffer) => {
                let _ = rl.add_history_entry(buffer.as_str());

                let mut token_map = parse(buffer.replace("\n", "").replace("\t", ""), &mut exp_map);
                let exp = read_from_tokens(&mut token_map).unwrap().into();
                let res = eval(exp, &env, &mut symbol_definitions).unwrap();
                println!("{}", res);
            }
            Err(ReadlineError::Interrupted) => {
                break;
            }
            Err(ReadlineError::Eof) => {
                break;
            }
            Err(_) => {
                break;
            }
        }
    }
    Ok(())
}

pub fn parse_and_run_scheme(buffer: String) {
    let builder = thread::Builder::new().name("evaluator".into()).stack_size(1024 * 1024 * 1024);

    let handler  = builder.spawn(||{
        let env = std_env();
        let mut exp_map: HashMap<String, VecDeque<String>> = HashMap::new();
        let mut symbol_definitions: HashMap<String, ExprKind> = HashMap::new();
        let mut token_map = parse(buffer, &mut exp_map);

        while token_map.len() > 0 {
            let sym_exp = read_from_tokens(&mut token_map).unwrap();
            let exp = sym_exp.clone().into();
            let res = eval(exp, &env, &mut symbol_definitions).unwrap();
            println!("{}", res);
        }
    }).unwrap();

    handler.join().unwrap();
}
