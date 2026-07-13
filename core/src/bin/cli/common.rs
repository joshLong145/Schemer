use rustyline::error::ReadlineError;
use rustyline::DefaultEditor;
use schemer::eval::{eval_source, Session};
use schemer::parser::read_all;
use schemer::types::Value;

/// REPL mode (spec §8): one long-lived `Session` (which owns its
/// `AnfTransformer`) for the whole process, one `transform_program` call per
/// line via `Session::eval_program`, evaluating only that line's `entry` -
/// earlier lines are never re-transformed or re-evaluated.
/// `Session.functions`/`globals` accumulate across lines, so a `define` on
/// one line is visible on a later one. The rejected alternative -
/// re-transforming the full input history on every line - replays every
/// prior side effect (§8/§11 item 3).
pub fn repl() -> rustyline::Result<()> {
    let mut session = Session::new();
    load_prelude(&mut session);

    let mut rl = DefaultEditor::new()?;

    loop {
        let readline = rl.readline("ƛ > ");
        match readline {
            Ok(buffer) => {
                let _ = rl.add_history_entry(buffer.as_str());

                let input = buffer.replace('\n', "").replace('\t', "");
                match read_all(&input) {
                    Ok(exprs) => match session.eval_program(exprs) {
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

/// Single-shot file execution (spec §2/§9 step 8): one `Session`, prelude
/// first then the user's source, each via one `eval_source` call (safe on
/// one `Session` because it owns a persistent `AnfTransformer`, so
/// prelude-defined names intern to the same `Symbol`s the user program
/// references). The dedicated 1GB-stack evaluator thread is gone - it
/// existed only to paper over the old evaluator's native-Rust-recursion
/// tail calls; the ANF interpreter's `apply_tail` trampoline gives O(1)
/// Rust-stack tail calls, so recursion depth is bounded by source-level
/// nesting, not loop iteration count.
pub fn parse_and_run_scheme(buffer: String) {
    let mut session = Session::new();
    load_prelude(&mut session);

    match eval_source(&buffer, &mut session) {
        Ok(res) => {
            // Only print the final result if it's not Void
            if !matches!(res, Value::Void) {
                println!("{}", res);
            }
        }
        Err(e) => eprintln!("Error: {}", e),
    }
}

/// Load `lib/prelude.scm` (relative to cwd, matching `compile_file`'s
/// convention) into `session`. Silently skipped if the file doesn't exist;
/// a prelude that exists but fails to evaluate is reported to stderr
/// without aborting, since user code may not need it.
fn load_prelude(session: &mut Session) {
    let prelude = std::fs::read_to_string("lib/prelude.scm").unwrap_or_default();
    if prelude.is_empty() {
        return;
    }
    if let Err(e) = eval_source(&prelude, session) {
        eprintln!("Warning: failed to load prelude: {}", e);
    }
}
