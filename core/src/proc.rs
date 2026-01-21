use std::collections::HashMap;

use crate::types::Value;

/// Type alias for built-in procedure functions
pub type ProcedureFn =
    Box<dyn Fn(Vec<Value>, &mut HashMap<String, Value>) -> Result<Value, String>>;
