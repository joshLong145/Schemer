use crate::types::Value;
use std::sync::Arc;

/// A cons cell (pair) containing two values
#[derive(Clone, Debug, PartialEq)]
pub struct Pair {
    pub car: Option<Arc<Value>>,
    pub cdr: Option<Arc<Value>>,
}

impl Pair {
    /// Create a new pair (cons cell) with car and cdr
    /// Equivalent to Scheme: (cons car cdr)
    pub fn new(car: Value, cdr: Value) -> Self {
        Pair {
            car: Some(Arc::new(car)),
            cdr: Some(Arc::new(cdr)),
        }
    }

    /// Get a reference to the car value
    pub fn car_ref(&self) -> Option<&Value> {
        self.car.as_ref().map(|arc| arc.as_ref())
    }

    /// Get a reference to the cdr value
    pub fn cdr_ref(&self) -> Option<&Value> {
        self.cdr.as_ref().map(|arc| arc.as_ref())
    }

    /// Get the car value (consuming self)
    pub fn car(self) -> Option<Arc<Value>> {
        self.car
    }

    /// Get the cdr value (consuming self)
    pub fn cdr(self) -> Option<Arc<Value>> {
        self.cdr
    }

    /// Check if this pair is a proper list (nil-terminated)
    pub fn is_list(&self) -> bool {
        let mut current_cdr = self.cdr.clone();

        // Traverse to the end of the pair chain
        while let Some(cdr) = current_cdr {
            match cdr.as_ref() {
                Value::Pair(p) => {
                    current_cdr = Some(Arc::new(p.1.clone()));
                }
                Value::Nil => return true,
                _ => return false,
            }
        }

        // Terminated with None (nil)
        true
    }
}

impl Iterator for Pair {
    type Item = Arc<Value>;

    fn next(&mut self) -> Option<Self::Item> {
        let curr = self.car.clone();

        match self.cdr.clone() {
            Some(cdr) => match cdr.as_ref() {
                Value::Pair(p) => {
                    self.car = Some(Arc::new(p.0.clone()));
                    self.cdr = Some(Arc::new(p.1.clone()));
                }
                Value::Nil => {
                    self.car = None;
                    self.cdr = None;
                }
                _ => {
                    self.car = None;
                    self.cdr = None;
                }
            },
            None => {
                self.car = None;
                self.cdr = None;
            }
        }

        curr
    }
}

impl std::fmt::Display for Pair {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match &self.car {
            None => write!(f, "()"),
            Some(car) => match &self.cdr {
                None => {
                    // Single element list: (car)
                    write!(f, "({})", car)
                }
                Some(cdr) => {
                    if self.is_list() {
                        // Proper list: format elements without dot notation
                        write!(f, "(")?;
                        write!(f, "{}", car)?;
                        // Traverse the rest of the list
                        let mut current = cdr.clone();
                        loop {
                            match current.as_ref() {
                                Value::Pair(p) => {
                                    write!(f, " {}", p.0)?;
                                    current = Arc::new(p.1.clone());
                                }
                                Value::Nil => break,
                                other => {
                                    write!(f, " . {}", other)?;
                                    break;
                                }
                            }
                        }
                        write!(f, ")")
                    } else {
                        // Improper list: (car . cdr)
                        write!(f, "({} . {})", car, cdr)
                    }
                }
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::value::Number;

    #[test]
    fn test_new_pair() {
        let pair = Pair::new(Value::Number(Number::Int(1)), Value::Number(Number::Int(2)));

        assert_eq!(pair.car_ref(), Some(&Value::Number(Number::Int(1))));
        assert_eq!(pair.cdr_ref(), Some(&Value::Number(Number::Int(2))));
    }

    #[test]
    fn test_car_cdr() {
        let pair = Pair::new(Value::Number(Number::Int(1)), Value::Number(Number::Int(2)));

        let car = pair.clone().car();
        assert!(car.is_some());
        assert_eq!(car.unwrap().as_ref(), &Value::Number(Number::Int(1)));

        let cdr = pair.cdr();
        assert!(cdr.is_some());
        assert_eq!(cdr.unwrap().as_ref(), &Value::Number(Number::Int(2)));
    }

    #[test]
    fn test_is_list_proper() {
        // Proper list: (1 . (2 . nil))
        let pair = Pair::new(
            Value::Number(Number::Int(1)),
            Value::Pair(Arc::new((Value::Number(Number::Int(2)), Value::Nil))),
        );

        assert!(pair.is_list());
    }

    #[test]
    fn test_is_list_improper() {
        // Improper list (dotted pair): (1 . 2)
        let pair = Pair::new(Value::Number(Number::Int(1)), Value::Number(Number::Int(2)));

        assert!(!pair.is_list());
    }

    #[test]
    fn test_display_single_element() {
        let pair = Pair {
            car: Some(Arc::new(Value::Number(Number::Int(1)))),
            cdr: None,
        };

        let display = format!("{}", pair);
        assert_eq!(display, "(1)");
    }

    #[test]
    fn test_display_dotted_pair() {
        let pair = Pair::new(Value::Number(Number::Int(1)), Value::Number(Number::Int(2)));

        let display = format!("{}", pair);
        assert!(display.contains("."));
        assert!(display.contains("1"));
        assert!(display.contains("2"));
    }

    #[test]
    fn test_is_list_with_nil_terminator() {
        // Proper list with nil terminator: (1 . nil)
        let pair = Pair::new(Value::Number(Number::Int(1)), Value::Nil);
        assert!(pair.is_list());
    }
}
