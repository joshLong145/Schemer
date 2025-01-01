use crate::types::{Atom, RLispNumber, SymbolicExpression};

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

        r.sub(l)
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
