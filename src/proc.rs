use std::collections::HashMap;

use crate::{
    eval::eval,
    types::{RLispSubSymbolicExpressions, SymbolicExpression},
};

pub struct Proc<'a> {
    pub params: HashMap<String, SymbolicExpression>,
    pub signature: SymbolicExpression,
    pub body: SymbolicExpression,
    pub env: &'a HashMap<
        String,
        Box<dyn Fn(RLispSubSymbolicExpressions, &mut HashMap<String, SymbolicExpression>) -> Result<SymbolicExpression, String>>,
    >,
}

impl std::fmt::Display for Proc<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let fmt_str = format!(
            "\nsignature {}\nparameters {}\nbody {}",
            self.signature,
            self.params.clone().into_keys().collect::<String>(),
            self.body
        );
        write!(f, "{}", fmt_str)
    }
}

pub trait Eval {
    fn proc_eval(
        &self,
        symbol_definitions: &mut HashMap<String, SymbolicExpression>,
    ) -> Result<SymbolicExpression, String>;
}

impl Eval for Proc<'_> {
    fn proc_eval(
        &self,
        symbol_definitions: &mut HashMap<String, SymbolicExpression>,
    ) -> Result<SymbolicExpression, String> {
        let mut local_symbols: HashMap<String, SymbolicExpression> = HashMap::new();

        // Add all existing symbol definitions (including recursive function definitions)
        local_symbols.extend(symbol_definitions.clone());

        // Override with parameter bindings
        local_symbols.extend(self.params.clone());

        eval(&self.body, self.env, &mut local_symbols)
    }
}
