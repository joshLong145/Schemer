//! ANF-based Scheme evaluator: consumes the *same* `AnfProgram` the compiler
//! produces (see `docs/design/anf-interpreter-spec.md`), so special-form
//! semantics are defined once, in `AnfTransformer`, for both backends.
//! (The previous `Value`-walking evaluator, `core/src/eval.rs`, was deleted
//! in rollout step 9 once all callers migrated here.)

pub mod interp;
pub mod primitives;
pub mod session;
pub mod value_bridge;

pub use session::Session;

use crate::parser::read_all;
use crate::types::Value;

/// Parse, ANF-transform, and evaluate a whole program's source, threading
/// state through `session`.
pub fn eval_source(src: &str, session: &mut Session) -> Result<Value, String> {
    let exprs = read_all(src).map_err(|e| e.msg)?;
    session.eval_program(exprs)
}
