use std::{cell::RefCell, collections::HashMap, rc::Rc};

use crate::{
    eval::eval,
    types::{RLispSubSymbolicExpressions, SymbolicExpression},
};

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct Proc {
    pub params: HashMap<String, SymbolicExpression>,
    pub signature: SymbolicExpression,
    pub body: SymbolicExpression,
    pub env: Rc<RefCell<HashMap<
        String,
        Box<dyn Fn(RLispSubSymbolicExpressions) -> Result<SymbolicExpression, String>>,
    >>>,
}

impl std::fmt::Display for Proc {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let fmt_str = format!(
            "lambda: {} {}",
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

impl Eval for Proc {
    fn proc_eval(
        &self,
        symbol_definitions: &mut HashMap<String, SymbolicExpression>,
    ) -> Result<SymbolicExpression, String> {
        let mut symbols: HashMap<String, SymbolicExpression> = HashMap::new();
        symbols.extend(self.params.clone().into_iter());
        symbols.extend(symbol_definitions.clone().into_iter());

        eval(&self.body, &*self.env.borrow(), &mut symbols)
    }
}
