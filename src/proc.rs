use std::collections::HashMap;

use crate::{
    eval::eval,
    types::{RLispSubSymbolicExpressions, SymbolicExpression},
};

pub struct Proc<'a> {
    pub params: HashMap<String, SymbolicExpression>,
    pub body: SymbolicExpression,
    pub env: &'a HashMap<
        String,
        Box<dyn Fn(RLispSubSymbolicExpressions) -> Result<SymbolicExpression, String>>,
    >,
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
        let mut symbols: HashMap<String, SymbolicExpression> = HashMap::new();
        symbols.extend(self.params.clone().into_iter());
        symbols.extend(symbol_definitions.clone().into_iter());

        eval(&self.body, self.env, &mut symbols)
    }
}
