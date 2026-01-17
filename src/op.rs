use crate::types::{Atom, Number, RLispNumber, SymbolicExpression, Value};
use std::cmp::Ordering;

pub trait NumericOps {
    fn add(&self, val: Atom) -> Result<SymbolicExpression, String>;

    fn sub(&self, val: Atom) -> Result<SymbolicExpression, String>;

    fn mul(&self, val: Atom) -> Result<SymbolicExpression, String>;

    fn div(&self, val: Atom) -> Result<SymbolicExpression, String>;
}

impl NumericOps for Atom {
    fn add(&self, val: Atom) -> Result<SymbolicExpression, String> {
        let l = match self {
            Atom::Number(rlisp_number) => rlisp_number,
            Atom::Symbol(_) => {
                return Err("invalid expression".into());
            }
            Atom::Bool(_) => {
                return Err("invalid expression".into());
            }
        };
        let r = match &val {
            Atom::Number(rlisp_number) => rlisp_number,
            Atom::Symbol(_) => {
                return Err("invalid expression".into());
            }
            Atom::Bool(_) => {
                return Err("invalid expression".into());
            }
        };

        l.add(r)
    }

    fn sub(&self, val: Atom) -> Result<SymbolicExpression, String> {
        let l = match self {
            Atom::Number(rlisp_number) => rlisp_number,
            Atom::Symbol(_) => {
                return Err("invalid expression".into());
            }
            Atom::Bool(_) => {
                return Err("invalid expression".into());
            }
        };
        let r = match &val {
            Atom::Number(rlisp_number) => rlisp_number,
            Atom::Symbol(_) => {
                return Err("invalid expression".into());
            }
            Atom::Bool(_) => {
                return Err("invalid expression".into());
            }
        };

        l.sub(r)
    }

    fn mul(&self, val: Atom) -> Result<SymbolicExpression, String> {
        let l = match self {
            Atom::Number(rlisp_number) => rlisp_number,
            Atom::Symbol(_) => {
                return Err("invalid expression".into());
            }
            Atom::Bool(_) => {
                return Err("invalid expression".into());
            }
        };
        let r = match &val {
            Atom::Number(rlisp_number) => rlisp_number,
            Atom::Symbol(_) => {
                return Err("invalid expression".into());
            }
            Atom::Bool(_) => {
                return Err("invalid expression".into());
            }
        };

        l.mul(r)
    }

    fn div(&self, val: Atom) -> Result<SymbolicExpression, String> {
        let l = match self {
            Atom::Number(rlisp_number) => rlisp_number,
            Atom::Symbol(_) => {
                return Err("invalid expression".into());
            }
            Atom::Bool(_) => {
                return Err("invalid expression".into());
            }
        };
        let r = match &val {
            Atom::Number(rlisp_number) => rlisp_number,
            Atom::Symbol(_) => {
                return Err("invalid expression".into());
            }
            Atom::Bool(_) => {
                return Err("invalid expression".into());
            }
        };

        l.div(r)
    }
}

impl RLispNumber {
    fn add(&self, val: &RLispNumber) -> Result<SymbolicExpression, String> {
        match self {
            RLispNumber::Int(r_i) => match val {
                RLispNumber::Int(l_i) => Ok(SymbolicExpression::Atom((r_i + l_i).to_string())),
                RLispNumber::Float(l_f) => {
                    let tmp = *r_i as f32;
                    Ok(SymbolicExpression::Atom((tmp + l_f).to_string()))
                }
            },
            RLispNumber::Float(r_f) => match val {
                RLispNumber::Int(l_i) => {
                    let tmp = *l_i as f32;
                    Ok(SymbolicExpression::Atom((r_f + tmp).to_string()))
                }
                RLispNumber::Float(l_f) => Ok(SymbolicExpression::Atom((r_f + l_f).to_string())),
            },
        }
    }

    fn sub(&self, val: &RLispNumber) -> Result<SymbolicExpression, String> {
        match self {
            RLispNumber::Int(r_i) => match val {
                RLispNumber::Int(l_i) => Ok(SymbolicExpression::Atom((r_i - l_i).to_string())),
                RLispNumber::Float(l_f) => {
                    let tmp = *r_i as f32;
                    Ok(SymbolicExpression::Atom((tmp - l_f).to_string()))
                }
            },
            RLispNumber::Float(r_f) => match val {
                RLispNumber::Int(l_i) => {
                    let tmp = *l_i as f32;
                    Ok(SymbolicExpression::Atom((r_f - tmp).to_string()))
                }
                RLispNumber::Float(l_f) => Ok(SymbolicExpression::Atom((r_f - l_f).to_string())),
            },
        }
    }

    fn div(&self, val: &RLispNumber) -> Result<SymbolicExpression, String> {
        match self {
            RLispNumber::Int(r_i) => match val {
                RLispNumber::Int(l_i) => Ok(SymbolicExpression::Atom((r_i / l_i).to_string())),
                RLispNumber::Float(l_f) => {
                    let tmp = *r_i as f32;
                    Ok(SymbolicExpression::Atom((tmp / l_f).to_string()))
                }
            },
            RLispNumber::Float(r_f) => match val {
                RLispNumber::Int(l_i) => {
                    let tmp = *l_i as f32;
                    Ok(SymbolicExpression::Atom((r_f / tmp).to_string()))
                }
                RLispNumber::Float(l_f) => Ok(SymbolicExpression::Atom((r_f / l_f).to_string())),
            },
        }
    }

    fn mul(&self, val: &RLispNumber) -> Result<SymbolicExpression, String> {
        match self {
            RLispNumber::Int(r_i) => match val {
                RLispNumber::Int(l_i) => Ok(SymbolicExpression::Atom((r_i * l_i).to_string())),
                RLispNumber::Float(l_f) => {
                    let tmp = *r_i as f32;
                    Ok(SymbolicExpression::Atom((tmp * l_f).to_string()))
                }
            },
            RLispNumber::Float(r_f) => match val {
                RLispNumber::Int(l_i) => {
                    let tmp = *l_i as f32;
                    Ok(SymbolicExpression::Atom((r_f * tmp).to_string()))
                }
                RLispNumber::Float(l_f) => Ok(SymbolicExpression::Atom((r_f * l_f).to_string())),
            },
        }
    }
}

impl PartialOrd for Atom {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        if self == other {
            return Some(Ordering::Equal);
        }

        match (self, other) {
            (Atom::Number(l), Atom::Number(r)) => l.partial_cmp(r),
            (Atom::Number(_), Atom::Symbol(_)) => todo!(),
            (Atom::Number(_), Atom::Bool(_)) => todo!(),
            (Atom::Symbol(_), Atom::Number(_)) => todo!(),
            (Atom::Symbol(_), Atom::Symbol(_)) => todo!(),
            (Atom::Symbol(_), Atom::Bool(_)) => todo!(),
            (Atom::Bool(_), Atom::Number(_)) => todo!(),
            (Atom::Bool(_), Atom::Symbol(_)) => todo!(),
            (Atom::Bool(_), Atom::Bool(_)) => todo!(),
        }
    }
}

impl Eq for Atom {}
impl PartialEq for RLispNumber {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Self::Int(l0), Self::Int(r0)) => l0 == r0,
            (Self::Float(l0), Self::Float(r0)) => l0 == r0,
            _ => false,
        }
    }
}

impl PartialOrd for RLispNumber {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        let lhs = match self {
            RLispNumber::Int(i) => *i as f32,
            RLispNumber::Float(f) => f.to_owned(),
        };

        let rhs = match other {
            RLispNumber::Int(i) => *i as f32,
            RLispNumber::Float(f) => f.to_owned(),
        };

        if lhs > rhs {
            return Some(Ordering::Greater);
        }

        if lhs < rhs {
            return Some(Ordering::Less);
        }

        Some(Ordering::Equal)
    }
}

impl PartialEq for crate::types::RLispBoolean {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Self::True(l0), Self::True(r0)) => l0 == r0,
            (Self::False(l0), Self::False(r0)) => l0 == r0,
            _ => false,
        }
    }
}

impl PartialEq for Atom {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Self::Number(l0), Self::Number(r0)) => l0 == r0,
            (Self::Symbol(l0), Self::Symbol(r0)) => l0 == r0,
            (Self::Bool(l0), Self::Bool(r0)) => l0 == r0,
            _ => false,
        }
    }
}

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
