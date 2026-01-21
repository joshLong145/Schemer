use rustyline::error::ReadlineError;
use rustyline::DefaultEditor;
use schemer::env::{std_const_exp, std_env};
use schemer::eval::eval_value;
use schemer::parser::{read, read_all};

use std::thread;

pub fn repl() -> rustyline::Result<()> {
    let env = std_env();
    let mut symbol_definitions = std_const_exp();

    let mut rl = DefaultEditor::new()?;

    loop {
        let readline = rl.readline("ƛ > ");
        match readline {
            Ok(buffer) => {
                let _ = rl.add_history_entry(buffer.as_str());

                let input = buffer.replace("\n", "").replace("\t", "");
                match read(&input) {
                    Ok(expr) => match eval_value(expr, &env, &mut symbol_definitions) {
                        Ok(res) => println!("{}", res),
                        Err(e) => eprintln!("Error: {}", e),
                    },
                    Err(e) => eprintln!("Parse error: {}", e.msg),
                }
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
    let builder = thread::Builder::new()
        .name("evaluator".into())
        .stack_size(1024 * 1024 * 1024);

    let handler = builder
        .spawn(move || {
            let env = std_env();
            let mut symbol_definitions = std_const_exp();

            match read_all(&buffer) {
                Ok(exprs) => {
                    let mut last_result = Ok(schemer::types::Value::Void);
                    for expr in exprs {
                        last_result = eval_value(expr, &env, &mut symbol_definitions);
                        if let Err(ref e) = last_result {
                            eprintln!("Error: {}", e);
                            return;
                        }
                    }
                    // Only print the final result if it's not Void
                    if let Ok(res) = last_result {
                        if !matches!(res, schemer::types::Value::Void) {
                            println!("{}", res);
                        }
                    }
                }
                Err(e) => eprintln!("Parse error: {}", e.msg),
            }
        })
        .unwrap();

    handler.join().unwrap();
}
