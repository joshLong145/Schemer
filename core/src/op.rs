use crate::types::{Number, Value};
use std::cmp::Ordering;

// Value-based numeric operations
pub trait ValueNumericOps {
    fn add(&self, other: &Value) -> Result<Value, String>;
    fn sub(&self, other: &Value) -> Result<Value, String>;
    fn mul(&self, other: &Value) -> Result<Value, String>;
    fn div(&self, other: &Value) -> Result<Value, String>;
}

impl ValueNumericOps for Value {
    fn add(&self, other: &Value) -> Result<Value, String> {
        match (self, other) {
            (Value::Number(l), Value::Number(r)) => Ok(Value::Number(l.add(r))),
            _ => Err("+ requires numeric arguments".to_string()),
        }
    }

    fn sub(&self, other: &Value) -> Result<Value, String> {
        match (self, other) {
            (Value::Number(l), Value::Number(r)) => Ok(Value::Number(l.sub(r))),
            _ => Err("- requires numeric arguments".to_string()),
        }
    }

    fn mul(&self, other: &Value) -> Result<Value, String> {
        match (self, other) {
            (Value::Number(l), Value::Number(r)) => Ok(Value::Number(l.mul(r))),
            _ => Err("* requires numeric arguments".to_string()),
        }
    }

    fn div(&self, other: &Value) -> Result<Value, String> {
        match (self, other) {
            (Value::Number(l), Value::Number(r)) => l.div(r).map(Value::Number),
            _ => Err("/ requires numeric arguments".to_string()),
        }
    }
}

impl Number {
    pub fn add(&self, other: &Number) -> Number {
        match (self, other) {
            (Number::Int(a), Number::Int(b)) => Number::Int(a + b),
            (Number::Int(a), Number::Float(b)) => Number::Float(*a as f64 + b),
            (Number::Float(a), Number::Int(b)) => Number::Float(a + *b as f64),
            (Number::Float(a), Number::Float(b)) => Number::Float(a + b),
        }
    }

    pub fn sub(&self, other: &Number) -> Number {
        match (self, other) {
            (Number::Int(a), Number::Int(b)) => Number::Int(a - b),
            (Number::Int(a), Number::Float(b)) => Number::Float(*a as f64 - b),
            (Number::Float(a), Number::Int(b)) => Number::Float(a - *b as f64),
            (Number::Float(a), Number::Float(b)) => Number::Float(a - b),
        }
    }

    pub fn mul(&self, other: &Number) -> Number {
        match (self, other) {
            (Number::Int(a), Number::Int(b)) => Number::Int(a * b),
            (Number::Int(a), Number::Float(b)) => Number::Float(*a as f64 * b),
            (Number::Float(a), Number::Int(b)) => Number::Float(a * *b as f64),
            (Number::Float(a), Number::Float(b)) => Number::Float(a * b),
        }
    }

    pub fn div(&self, other: &Number) -> Result<Number, String> {
        match (self, other) {
            (_, Number::Int(0)) => Err("division by zero".to_string()),
            (_, Number::Float(b)) if *b == 0.0 => Err("division by zero".to_string()),
            (Number::Int(a), Number::Int(b)) => Ok(Number::Int(a / b)),
            (Number::Int(a), Number::Float(b)) => Ok(Number::Float(*a as f64 / b)),
            (Number::Float(a), Number::Int(b)) => Ok(Number::Float(a / *b as f64)),
            (Number::Float(a), Number::Float(b)) => Ok(Number::Float(a / b)),
        }
    }
}

impl PartialOrd for Number {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        let lhs = match self {
            Number::Int(i) => *i as f64,
            Number::Float(f) => *f,
        };
        let rhs = match other {
            Number::Int(i) => *i as f64,
            Number::Float(f) => *f,
        };
        lhs.partial_cmp(&rhs)
    }
}
