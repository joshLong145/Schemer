use crate::types::Value;
use std::fmt;
use std::sync::Arc;

/// A Scheme-style list built from Pairs of Values
/// Represents proper lists where the final cdr is nil (empty list)
#[derive(Clone, Debug, PartialEq)]
pub struct SchemeList {
    pub head: Option<Arc<(Value, Value)>>,
}

impl SchemeList {
    /// Create a new empty list (nil)
    pub fn new() -> Self {
        SchemeList { head: None }
    }

    /// Create an empty list (alias for new)
    pub fn nil() -> Self {
        Self::new()
    }

    /// Construct a new list by prepending an element (cons operation)
    /// cons(1, cons(2, nil)) creates the list (1 2)
    pub fn cons(car: Value, cdr: SchemeList) -> Self {
        let cdr_value = if cdr.is_empty() {
            Value::Nil
        } else {
            Value::List(Arc::new(cdr))
        };

        SchemeList {
            head: Some(Arc::new((car, cdr_value))),
        }
    }

    /// Check if the list is empty
    pub fn is_empty(&self) -> bool {
        self.head.is_none()
    }

    /// Get the first element of the list (car)
    pub fn car(&self) -> Option<&Value> {
        self.head.as_ref().map(|pair| &pair.0)
    }

    /// Get the rest of the list (cdr)
    pub fn cdr(&self) -> Option<SchemeList> {
        let head = self.head.as_ref()?;

        match &head.1 {
            Value::List(list) => Some((**list).clone()),
            Value::Nil => Some(SchemeList::nil()),
            _ => None,
        }
    }

    /// Get the length of the list
    pub fn length(&self) -> usize {
        let mut count = 0;
        let mut current = self.clone();

        while !current.is_empty() {
            count += 1;
            current = current.cdr().unwrap_or_else(SchemeList::nil);
        }

        count
    }

    /// Get the nth element of the list (0-indexed)
    /// Returns a clone of the value at the given index
    pub fn nth(&self, index: usize) -> Option<Value> {
        let mut current = self.clone();
        let mut i = 0;

        while i < index {
            current = current.cdr()?;
            i += 1;
        }

        current.car().cloned()
    }

    /// Reverse the list
    pub fn reverse(&self) -> SchemeList {
        let mut result = SchemeList::nil();
        let mut current = self.clone();

        while !current.is_empty() {
            if let Some(car) = current.car() {
                result = SchemeList::cons(car.clone(), result);
            }
            current = current.cdr().unwrap_or_else(SchemeList::nil);
        }

        result
    }

    /// Build a SchemeList from a vector of Value
    pub fn from_vec(vec: Vec<Value>) -> SchemeList {
        let mut result = SchemeList::nil();

        // Build list in reverse order
        for val in vec.into_iter().rev() {
            result = SchemeList::cons(val, result);
        }

        result
    }

    /// Convert the SchemeList to a vector of Value
    pub fn to_vec(&self) -> Vec<Value> {
        let mut vec = Vec::new();
        let mut current = self.clone();

        while !current.is_empty() {
            if let Some(car) = current.car() {
                vec.push(car.clone());
            }
            current = current.cdr().unwrap_or_else(SchemeList::nil);
        }

        vec
    }
}

impl Default for SchemeList {
    fn default() -> Self {
        Self::new()
    }
}

impl fmt::Display for SchemeList {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.is_empty() {
            return write!(f, "()");
        }

        write!(f, "(")?;

        let mut current = self.clone();
        let mut first = true;

        while !current.is_empty() {
            if !first {
                write!(f, " ")?;
            }
            first = false;

            if let Some(car) = current.car() {
                write!(f, "{}", car)?;
            }

            current = current.cdr().unwrap_or_else(SchemeList::nil);
        }

        write!(f, ")")
    }
}

/// Iterator over SchemeList elements
pub struct SchemeListIter {
    current: SchemeList,
}

impl Iterator for SchemeListIter {
    type Item = Value;

    fn next(&mut self) -> Option<Self::Item> {
        let car = self.current.car()?.clone();
        self.current = self.current.cdr().unwrap_or_else(SchemeList::nil);
        Some(car)
    }
}

impl IntoIterator for SchemeList {
    type Item = Value;
    type IntoIter = SchemeListIter;

    fn into_iter(self) -> Self::IntoIter {
        SchemeListIter { current: self }
    }
}

impl IntoIterator for &SchemeList {
    type Item = Value;
    type IntoIter = SchemeListIter;

    fn into_iter(self) -> Self::IntoIter {
        SchemeListIter {
            current: self.clone(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::value::Number;

    #[test]
    fn test_empty_list() {
        let empty = SchemeList::new();
        assert!(empty.is_empty());
        assert_eq!(empty.length(), 0);
        assert!(empty.car().is_none());
        assert!(empty.cdr().is_none());
    }

    #[test]
    fn test_nil() {
        let nil = SchemeList::nil();
        assert!(nil.is_empty());
        assert_eq!(nil, SchemeList::new());
    }

    #[test]
    fn test_cons_single_element() {
        let list = SchemeList::cons(Value::Number(Number::Int(1)), SchemeList::nil());

        assert!(!list.is_empty());
        assert_eq!(list.length(), 1);
        assert_eq!(list.car(), Some(&Value::Number(Number::Int(1))));
    }

    #[test]
    fn test_cons_multiple_elements() {
        // Build list (1 2 3)
        let list = SchemeList::cons(
            Value::Number(Number::Int(1)),
            SchemeList::cons(
                Value::Number(Number::Int(2)),
                SchemeList::cons(Value::Number(Number::Int(3)), SchemeList::nil()),
            ),
        );

        assert_eq!(list.length(), 3);
    }

    #[test]
    fn test_car_cdr() {
        // Build list (1 2 3)
        let list = SchemeList::cons(
            Value::Number(Number::Int(1)),
            SchemeList::cons(
                Value::Number(Number::Int(2)),
                SchemeList::cons(Value::Number(Number::Int(3)), SchemeList::nil()),
            ),
        );

        // Test car
        assert_eq!(list.car(), Some(&Value::Number(Number::Int(1))));

        // Test cdr
        let cdr = list.cdr().unwrap();
        assert_eq!(cdr.length(), 2);
        assert_eq!(cdr.car(), Some(&Value::Number(Number::Int(2))));
    }

    #[test]
    fn test_length() {
        let empty = SchemeList::nil();
        assert_eq!(empty.length(), 0);

        let one = SchemeList::cons(Value::Number(Number::Int(1)), SchemeList::nil());
        assert_eq!(one.length(), 1);

        let three = SchemeList::cons(
            Value::Number(Number::Int(1)),
            SchemeList::cons(
                Value::Number(Number::Int(2)),
                SchemeList::cons(Value::Number(Number::Int(3)), SchemeList::nil()),
            ),
        );
        assert_eq!(three.length(), 3);
    }

    #[test]
    fn test_nth() {
        let list = SchemeList::cons(
            Value::Number(Number::Int(10)),
            SchemeList::cons(
                Value::Number(Number::Int(20)),
                SchemeList::cons(Value::Number(Number::Int(30)), SchemeList::nil()),
            ),
        );

        assert_eq!(list.nth(0), Some(Value::Number(Number::Int(10))));
        assert_eq!(list.nth(1), Some(Value::Number(Number::Int(20))));
        assert_eq!(list.nth(2), Some(Value::Number(Number::Int(30))));
        assert!(list.nth(3).is_none());
    }

    #[test]
    fn test_from_vec() {
        let vec = vec![
            Value::Number(Number::Int(1)),
            Value::Number(Number::Int(2)),
            Value::Number(Number::Int(3)),
        ];

        let list = SchemeList::from_vec(vec);
        assert_eq!(list.length(), 3);
        assert_eq!(list.nth(0), Some(Value::Number(Number::Int(1))));
    }

    #[test]
    fn test_to_vec() {
        let list = SchemeList::cons(
            Value::Number(Number::Int(1)),
            SchemeList::cons(
                Value::Number(Number::Int(2)),
                SchemeList::cons(Value::Number(Number::Int(3)), SchemeList::nil()),
            ),
        );

        let vec = list.to_vec();
        assert_eq!(vec.len(), 3);
        assert_eq!(vec[0], Value::Number(Number::Int(1)));
        assert_eq!(vec[1], Value::Number(Number::Int(2)));
        assert_eq!(vec[2], Value::Number(Number::Int(3)));
    }

    #[test]
    fn test_vec_roundtrip() {
        let original_vec = vec![
            Value::Number(Number::Int(1)),
            Value::Number(Number::Int(2)),
            Value::Number(Number::Int(3)),
        ];

        let list = SchemeList::from_vec(original_vec.clone());
        let result_vec = list.to_vec();

        assert_eq!(original_vec, result_vec);
    }

    #[test]
    fn test_reverse() {
        let list = SchemeList::cons(
            Value::Number(Number::Int(1)),
            SchemeList::cons(
                Value::Number(Number::Int(2)),
                SchemeList::cons(Value::Number(Number::Int(3)), SchemeList::nil()),
            ),
        );

        let reversed = list.reverse();
        assert_eq!(reversed.length(), 3);

        let vec = reversed.to_vec();
        assert_eq!(vec[0], Value::Number(Number::Int(3)));
        assert_eq!(vec[1], Value::Number(Number::Int(2)));
        assert_eq!(vec[2], Value::Number(Number::Int(1)));
    }

    #[test]
    fn test_reverse_empty() {
        let empty = SchemeList::nil();
        let reversed = empty.reverse();
        assert!(reversed.is_empty());
    }

    #[test]
    fn test_display_empty() {
        let empty = SchemeList::nil();
        assert_eq!(format!("{}", empty), "()");
    }

    #[test]
    fn test_display_single() {
        let list = SchemeList::cons(Value::Number(Number::Int(42)), SchemeList::nil());
        assert_eq!(format!("{}", list), "(42)");
    }

    #[test]
    fn test_display_multiple() {
        let list = SchemeList::cons(
            Value::Number(Number::Int(1)),
            SchemeList::cons(
                Value::Number(Number::Int(2)),
                SchemeList::cons(Value::Number(Number::Int(3)), SchemeList::nil()),
            ),
        );
        assert_eq!(format!("{}", list), "(1 2 3)");
    }

    #[test]
    fn test_iterator() {
        let list = SchemeList::from_vec(vec![
            Value::Number(Number::Int(1)),
            Value::Number(Number::Int(2)),
            Value::Number(Number::Int(3)),
        ]);

        let collected: Vec<Value> = list.into_iter().collect();
        assert_eq!(collected.len(), 3);
        assert_eq!(collected[0], Value::Number(Number::Int(1)));
        assert_eq!(collected[1], Value::Number(Number::Int(2)));
        assert_eq!(collected[2], Value::Number(Number::Int(3)));
    }
}
