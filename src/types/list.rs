use crate::types::pair::Pair;
use crate::types::ExprKind;
use std::fmt;
use std::sync::Arc;

/// A Scheme-style list built from Pairs (cons cells)
/// Represents proper lists where the final cdr is nil (empty list)
#[derive(Clone, Debug, PartialEq)]
pub struct PairList {
    pub head: Option<Arc<Pair<ExprKind>>>,
}

impl PairList {
    /// Create a new empty list (nil)
    pub fn new() -> Self {
        PairList { head: None }
    }

    /// Create an empty list (alias for new)
    pub fn nil() -> Self {
        Self::new()
    }

    /// Construct a new list by prepending an element (cons operation)
    /// cons(1, cons(2, nil)) creates the list (1 2)
    pub fn cons(car: ExprKind, cdr: PairList) -> Self {
        let pair = Pair {
            car: Some(Arc::new(car)),
            cdr: cdr.head.map(|pair| Arc::new(ExprKind::Pair(pair))),
        };

        PairList {
            head: Some(Arc::new(pair)),
        }
    }

    /// Check if the list is empty
    pub fn is_empty(&self) -> bool {
        self.head.is_none()
    }

    /// Get the first element of the list (car)
    pub fn car(&self) -> Option<Arc<ExprKind>> {
        self.head.as_ref()?.car.clone()
    }

    /// Get the rest of the list (cdr)
    pub fn cdr(&self) -> Option<PairList> {
        let head = self.head.as_ref()?;

        match &head.cdr {
            Some(cdr_expr) => match cdr_expr.as_ref() {
                ExprKind::Pair(pair) => Some(PairList {
                    head: Some(pair.clone()),
                }),
                _ => None,
            },
            None => Some(PairList::nil()),
        }
    }
}

impl Default for PairList {
    fn default() -> Self {
        Self::new()
    }
}

impl PairList {
    /// Get the length of the list
    pub fn length(&self) -> usize {
        let mut count = 0;
        let mut current = self.clone();

        while !current.is_empty() {
            count += 1;
            current = current.cdr().unwrap_or_else(PairList::nil);
        }

        count
    }

    /// Get the nth element of the list (0-indexed)
    pub fn nth(&self, index: usize) -> Option<Arc<ExprKind>> {
        let mut current = self.clone();
        let mut i = 0;

        while i < index {
            current = current.cdr()?;
            i += 1;
        }

        current.car()
    }

    /// Append another list to the end of this list
    pub fn append(&self, other: &PairList) -> Result<PairList, String> {
        if self.is_empty() {
            return Ok(other.clone());
        }

        let car = self
            .car()
            .ok_or_else(|| "Invalid list structure: non-empty list has no car".to_string())?;
        let cdr = self.cdr().unwrap_or_else(PairList::nil);

        Ok(PairList::cons((*car).clone(), cdr.append(other)?))
    }

    /// Reverse the list
    pub fn reverse(&self) -> PairList {
        let mut result = PairList::nil();
        let mut current = self.clone();

        while !current.is_empty() {
            if let Some(car) = current.car() {
                result = PairList::cons((*car).clone(), result);
            }
            current = current.cdr().unwrap_or_else(PairList::nil);
        }

        result
    }

    /// Build a PairList from a vector of ExprKind
    pub fn from_vec(vec: Vec<ExprKind>) -> PairList {
        let mut result = PairList::nil();

        // Build list in reverse order, then reverse it
        for expr in vec.into_iter().rev() {
            result = PairList::cons(expr, result);
        }

        result
    }

    /// Convert the PairList to a vector of ExprKind
    pub fn to_vec(&self) -> Vec<ExprKind> {
        let mut vec = Vec::new();
        let mut current = self.clone();

        while !current.is_empty() {
            if let Some(car) = current.car() {
                vec.push((*car).clone());
            }
            current = current.cdr().unwrap_or_else(PairList::nil);
        }

        vec
    }
}

impl fmt::Display for PairList {
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

            current = current.cdr().unwrap_or_else(PairList::nil);
        }

        write!(f, ")")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::{Atom, RLispNumber};

    #[test]
    fn test_empty_list() {
        let empty = PairList::new();
        assert!(empty.is_empty());
        assert_eq!(empty.length(), 0);
        assert!(empty.car().is_none());
        assert!(empty.cdr().is_none());
    }

    #[test]
    fn test_nil() {
        let nil = PairList::nil();
        assert!(nil.is_empty());
        assert_eq!(nil, PairList::new());
    }

    #[test]
    fn test_cons_single_element() {
        let list = PairList::cons(
            ExprKind::Atom(Arc::new(Atom::Number(RLispNumber::Int(1)))),
            PairList::nil(),
        );

        assert!(!list.is_empty());
        assert_eq!(list.length(), 1);

        let car = list.car().unwrap();
        assert_eq!(
            car.as_ref(),
            &ExprKind::Atom(Arc::new(Atom::Number(RLispNumber::Int(1))))
        );
    }

    #[test]
    fn test_cons_multiple_elements() {
        // Build list (1 2 3)
        let list = PairList::cons(
            ExprKind::Atom(Arc::new(Atom::Number(RLispNumber::Int(1)))),
            PairList::cons(
                ExprKind::Atom(Arc::new(Atom::Number(RLispNumber::Int(2)))),
                PairList::cons(
                    ExprKind::Atom(Arc::new(Atom::Number(RLispNumber::Int(3)))),
                    PairList::nil(),
                ),
            ),
        );

        assert_eq!(list.length(), 3);
    }

    #[test]
    fn test_car_cdr() {
        // Build list (1 2 3)
        let list = PairList::cons(
            ExprKind::Atom(Arc::new(Atom::Number(RLispNumber::Int(1)))),
            PairList::cons(
                ExprKind::Atom(Arc::new(Atom::Number(RLispNumber::Int(2)))),
                PairList::cons(
                    ExprKind::Atom(Arc::new(Atom::Number(RLispNumber::Int(3)))),
                    PairList::nil(),
                ),
            ),
        );

        // Test car
        let car = list.car().unwrap();
        assert_eq!(
            car.as_ref(),
            &ExprKind::Atom(Arc::new(Atom::Number(RLispNumber::Int(1))))
        );

        // Test cdr
        let cdr = list.cdr().unwrap();
        assert_eq!(cdr.length(), 2);

        let cdr_car = cdr.car().unwrap();
        assert_eq!(
            cdr_car.as_ref(),
            &ExprKind::Atom(Arc::new(Atom::Number(RLispNumber::Int(2))))
        );
    }

    #[test]
    fn test_length() {
        let empty = PairList::nil();
        assert_eq!(empty.length(), 0);

        let one = PairList::cons(
            ExprKind::Atom(Arc::new(Atom::Number(RLispNumber::Int(1)))),
            PairList::nil(),
        );
        assert_eq!(one.length(), 1);

        let three = PairList::cons(
            ExprKind::Atom(Arc::new(Atom::Number(RLispNumber::Int(1)))),
            PairList::cons(
                ExprKind::Atom(Arc::new(Atom::Number(RLispNumber::Int(2)))),
                PairList::cons(
                    ExprKind::Atom(Arc::new(Atom::Number(RLispNumber::Int(3)))),
                    PairList::nil(),
                ),
            ),
        );
        assert_eq!(three.length(), 3);
    }

    #[test]
    fn test_nth() {
        let list = PairList::cons(
            ExprKind::Atom(Arc::new(Atom::Number(RLispNumber::Int(10)))),
            PairList::cons(
                ExprKind::Atom(Arc::new(Atom::Number(RLispNumber::Int(20)))),
                PairList::cons(
                    ExprKind::Atom(Arc::new(Atom::Number(RLispNumber::Int(30)))),
                    PairList::nil(),
                ),
            ),
        );

        let first = list.nth(0).unwrap();
        assert_eq!(
            first.as_ref(),
            &ExprKind::Atom(Arc::new(Atom::Number(RLispNumber::Int(10))))
        );

        let second = list.nth(1).unwrap();
        assert_eq!(
            second.as_ref(),
            &ExprKind::Atom(Arc::new(Atom::Number(RLispNumber::Int(20))))
        );

        let third = list.nth(2).unwrap();
        assert_eq!(
            third.as_ref(),
            &ExprKind::Atom(Arc::new(Atom::Number(RLispNumber::Int(30))))
        );

        assert!(list.nth(3).is_none());
    }

    #[test]
    fn test_from_vec() {
        let vec = vec![
            ExprKind::Atom(Arc::new(Atom::Number(RLispNumber::Int(1)))),
            ExprKind::Atom(Arc::new(Atom::Number(RLispNumber::Int(2)))),
            ExprKind::Atom(Arc::new(Atom::Number(RLispNumber::Int(3)))),
        ];

        let list = PairList::from_vec(vec);
        assert_eq!(list.length(), 3);

        let first = list.nth(0).unwrap();
        assert_eq!(
            first.as_ref(),
            &ExprKind::Atom(Arc::new(Atom::Number(RLispNumber::Int(1))))
        );
    }

    #[test]
    fn test_to_vec() {
        let list = PairList::cons(
            ExprKind::Atom(Arc::new(Atom::Number(RLispNumber::Int(1)))),
            PairList::cons(
                ExprKind::Atom(Arc::new(Atom::Number(RLispNumber::Int(2)))),
                PairList::cons(
                    ExprKind::Atom(Arc::new(Atom::Number(RLispNumber::Int(3)))),
                    PairList::nil(),
                ),
            ),
        );

        let vec = list.to_vec();
        assert_eq!(vec.len(), 3);
        assert_eq!(
            vec[0],
            ExprKind::Atom(Arc::new(Atom::Number(RLispNumber::Int(1))))
        );
        assert_eq!(
            vec[1],
            ExprKind::Atom(Arc::new(Atom::Number(RLispNumber::Int(2))))
        );
        assert_eq!(
            vec[2],
            ExprKind::Atom(Arc::new(Atom::Number(RLispNumber::Int(3))))
        );
    }

    #[test]
    fn test_vec_roundtrip() {
        let original_vec = vec![
            ExprKind::Atom(Arc::new(Atom::Number(RLispNumber::Int(1)))),
            ExprKind::Atom(Arc::new(Atom::Number(RLispNumber::Int(2)))),
            ExprKind::Atom(Arc::new(Atom::Number(RLispNumber::Int(3)))),
        ];

        let list = PairList::from_vec(original_vec.clone());
        let result_vec = list.to_vec();

        assert_eq!(original_vec, result_vec);
    }

    #[test]
    fn test_append() {
        let list1 = PairList::cons(
            ExprKind::Atom(Arc::new(Atom::Number(RLispNumber::Int(1)))),
            PairList::cons(
                ExprKind::Atom(Arc::new(Atom::Number(RLispNumber::Int(2)))),
                PairList::nil(),
            ),
        );

        let list2 = PairList::cons(
            ExprKind::Atom(Arc::new(Atom::Number(RLispNumber::Int(3)))),
            PairList::cons(
                ExprKind::Atom(Arc::new(Atom::Number(RLispNumber::Int(4)))),
                PairList::nil(),
            ),
        );

        let combined = list1.append(&list2).unwrap();
        assert_eq!(combined.length(), 4);

        let vec = combined.to_vec();
        assert_eq!(
            vec[0],
            ExprKind::Atom(Arc::new(Atom::Number(RLispNumber::Int(1))))
        );
        assert_eq!(
            vec[3],
            ExprKind::Atom(Arc::new(Atom::Number(RLispNumber::Int(4))))
        );
    }

    #[test]
    fn test_append_empty() {
        let list = PairList::cons(
            ExprKind::Atom(Arc::new(Atom::Number(RLispNumber::Int(1)))),
            PairList::nil(),
        );

        let result1 = list.append(&PairList::nil()).unwrap();
        assert_eq!(result1.length(), 1);

        let result2 = PairList::nil().append(&list).unwrap();
        assert_eq!(result2.length(), 1);
    }

    #[test]
    fn test_reverse() {
        let list = PairList::cons(
            ExprKind::Atom(Arc::new(Atom::Number(RLispNumber::Int(1)))),
            PairList::cons(
                ExprKind::Atom(Arc::new(Atom::Number(RLispNumber::Int(2)))),
                PairList::cons(
                    ExprKind::Atom(Arc::new(Atom::Number(RLispNumber::Int(3)))),
                    PairList::nil(),
                ),
            ),
        );

        let reversed = list.reverse();
        assert_eq!(reversed.length(), 3);

        let vec = reversed.to_vec();
        assert_eq!(
            vec[0],
            ExprKind::Atom(Arc::new(Atom::Number(RLispNumber::Int(3))))
        );
        assert_eq!(
            vec[1],
            ExprKind::Atom(Arc::new(Atom::Number(RLispNumber::Int(2))))
        );
        assert_eq!(
            vec[2],
            ExprKind::Atom(Arc::new(Atom::Number(RLispNumber::Int(1))))
        );
    }

    #[test]
    fn test_reverse_empty() {
        let empty = PairList::nil();
        let reversed = empty.reverse();
        assert!(reversed.is_empty());
    }

    #[test]
    fn test_display_empty() {
        let empty = PairList::nil();
        assert_eq!(format!("{}", empty), "()");
    }

    #[test]
    fn test_display_single() {
        let list = PairList::cons(
            ExprKind::Atom(Arc::new(Atom::Number(RLispNumber::Int(42)))),
            PairList::nil(),
        );
        assert_eq!(format!("{}", list), "(42)");
    }

    #[test]
    fn test_display_multiple() {
        let list = PairList::cons(
            ExprKind::Atom(Arc::new(Atom::Number(RLispNumber::Int(1)))),
            PairList::cons(
                ExprKind::Atom(Arc::new(Atom::Number(RLispNumber::Int(2)))),
                PairList::cons(
                    ExprKind::Atom(Arc::new(Atom::Number(RLispNumber::Int(3)))),
                    PairList::nil(),
                ),
            ),
        );
        assert_eq!(format!("{}", list), "(1 2 3)");
    }
}
