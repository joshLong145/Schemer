//! ANF-based Scheme evaluator.
//!
//! `legacy` is today's `Value`-walking evaluator (formerly `core/src/eval.rs`,
//! relocated here only because Rust doesn't allow both `eval.rs` and an
//! `eval/` directory module of the same name to coexist - its content is
//! unchanged and still fully usable via the re-export below). It is replaced
//! by the modules below, which evaluate the *same* `AnfProgram` the compiler
//! produces (see `docs/design/anf-interpreter-spec.md`) instead of
//! re-implementing special-form desugaring over raw `Value` S-expressions.
//! `legacy` is deleted once all callers migrate (rollout step 9 - not this
//! pass).

mod legacy;
pub use legacy::*;

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
