use crate::types::Number;
use std::cmp::Ordering;

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
