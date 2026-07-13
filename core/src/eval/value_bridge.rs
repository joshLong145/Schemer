//! `Atom` <-> `Value` conversions for literals crossing the ANF boundary.
//! `Atom::Var` is intentionally excluded from `atom_to_value` - resolving a
//! variable reference requires the environment/session, which is
//! `eval_atom`'s job (`interp.rs`), not a pure bridge function.

use std::sync::Arc;

use crate::compiler::anf::Atom;
use crate::eval::session::Session;
use crate::types::{Number, Value};

/// Convert a literal `Atom` to a `Value`, resolving `String`/`Symbol` table
/// indices against `session`. Errors on `Atom::Var` (see module docs).
pub fn atom_to_value(atom: &Atom, session: &Session) -> Result<Value, String> {
    match atom {
        Atom::Var(_) => Err("atom_to_value: Var must be resolved via the environment".to_string()),
        Atom::Int(i) => Ok(Value::Number(Number::Int(*i))),
        Atom::Float(f) => Ok(Value::Number(Number::Float(*f))),
        Atom::Bool(b) => Ok(Value::Boolean(*b)),
        Atom::Char(c) => Ok(Value::Char(*c)),
        Atom::String(idx) => session
            .strings
            .get(*idx)
            .map(|s| Value::String(Arc::new(s.clone())))
            .ok_or_else(|| format!("string literal index {} out of range", idx)),
        Atom::Symbol(idx) => session
            .symbols
            .get(*idx)
            .map(|s| Value::Symbol(s.clone()))
            .ok_or_else(|| format!("symbol literal index {} out of range", idx)),
        Atom::Nil => Ok(Value::Nil),
        Atom::Void => Ok(Value::Void),
    }
}

/// Best-effort reverse direction: not every `Value` maps back to a literal
/// `Atom` (e.g. `Pair`/`Closure`/`Box` have no literal-table representation).
/// Currently unused by the interpreter itself; kept for symmetry and for
/// future REPL-echoing use cases, per spec §2's module layout.
#[allow(dead_code)]
pub fn value_to_atom(value: &Value) -> Option<Atom> {
    match value {
        Value::Number(Number::Int(i)) => Some(Atom::Int(*i)),
        Value::Number(Number::Float(f)) => Some(Atom::Float(*f)),
        Value::Boolean(b) => Some(Atom::Bool(*b)),
        Value::Char(c) => Some(Atom::Char(*c)),
        Value::Nil => Some(Atom::Nil),
        Value::Void => Some(Atom::Void),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn literals_roundtrip() {
        let session = Session::new();
        assert_eq!(
            atom_to_value(&Atom::Int(42), &session).unwrap(),
            Value::Number(Number::Int(42))
        );
        assert_eq!(
            atom_to_value(&Atom::Bool(true), &session).unwrap(),
            Value::Boolean(true)
        );
        assert_eq!(atom_to_value(&Atom::Nil, &session).unwrap(), Value::Nil);
    }

    #[test]
    fn var_is_rejected() {
        let session = Session::new();
        let mut interner = crate::interner::Interner::new();
        let var = crate::compiler::anf::VarId::new("x", &mut interner);
        assert!(atom_to_value(&Atom::Var(var), &session).is_err());
    }
}
