use crate::types::{ExprKind, list::PairList};
use std::sync::Arc;

pub trait Node {}
impl Node for ExprKind {}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Pair<N: Node> {
    pub car: Option<Arc<N>>,
    pub cdr: Option<Arc<N>>,
}

impl Pair<ExprKind> {
    pub fn car(self) -> Option<Arc<ExprKind>> {
        self.car
    }

    pub fn cdr(self) -> Option<Arc<ExprKind>> {
        self.cdr
    }
}

impl Iterator for Pair<ExprKind> {
    type Item = Arc<ExprKind>;

    fn next(&mut self) -> Option<Self::Item> {
        let curr = self.car.clone();
        
        match self.cdr.clone() {
            Some(cdr) => match cdr.as_ref() {
                ExprKind::Pair(p) => {
                    self.car = p.car.clone();
                    self.cdr = p.cdr.clone();
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

impl std::fmt::Display for Pair<ExprKind> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let car = self.car.as_ref().unwrap();
        let cdr = self.cdr.as_ref().unwrap();
        
        if self.is_list() {
            write!(f, "({} {})", car, cdr)
        } else {
            write!(f, "({} . {})", car, cdr)
        }
    }
}

impl Pair<ExprKind> {
    pub fn is_list(&self) -> bool {
        let mut cursor = self.clone();

        // Traverse to the end of the pair chain
        while let Some(cdr) = cursor.cdr.clone() {
            match cdr.as_ref() {
                ExprKind::Pair(p) => {
                    cursor.car = p.car.clone();
                    cursor.cdr = p.cdr.clone();
                }
                ExprKind::Quote(q) => {
                    if let ExprKind::Pair(p) = &q.as_ref().expr {
                        cursor.car = p.car.clone();
                        cursor.cdr = p.cdr.clone();
                    } else {
                        break;
                    }
                }
                _ => break,
            }
        }

        // Check if the final cdr is a proper list terminator
        match cursor.cdr {
            None => false,
            Some(cdr) => match cdr.as_ref() {
                ExprKind::List(_) => true,
                ExprKind::Quote(q) => matches!(q.as_ref().expr, ExprKind::List(_)),
                _ => false,
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::types::{Atom, ExprKind, List, RLispNumber};
    use std::sync::Arc;

    #[test]
    fn test_new_pair_itr() {
        let pair = super::Pair::<ExprKind> {
            car: Some(Arc::new(ExprKind::Atom(Arc::new(Atom::Number(
                RLispNumber::Int(1),
            ))))),
            cdr: Some(Arc::new(ExprKind::Pair(Arc::new(super::Pair::<ExprKind> {
                car: Some(Arc::new(ExprKind::Atom(Arc::new(Atom::Number(
                    RLispNumber::Int(2),
                ))))),
                cdr: None,
            })))),
        };

        let mut i = 0;
        for p in pair.clone().into_iter() {
            if i == 0 {
                assert!(p.as_ref() == &ExprKind::Atom(Arc::new(Atom::Number(RLispNumber::Int(1)))));
            }

            if i == 1 {
                assert!(p.as_ref() == &ExprKind::Atom(Arc::new(Atom::Number(RLispNumber::Int(2)))));
            }

            i += 1;
        }
    }

    #[test]
    fn test_car_cdr() {
        let pair = super::Pair::<ExprKind> {
            car: Some(Arc::new(ExprKind::Atom(Arc::new(Atom::Number(
                RLispNumber::Int(1),
            ))))),
            cdr: Some(Arc::new(ExprKind::Atom(Arc::new(Atom::Number(
                RLispNumber::Int(2),
            ))))),
        };

        let car = pair.clone().car();
        assert!(car.is_some());
        assert_eq!(
            car.unwrap().as_ref(),
            &ExprKind::Atom(Arc::new(Atom::Number(RLispNumber::Int(1))))
        );

        let cdr = pair.cdr();
        assert!(cdr.is_some());
        assert_eq!(
            cdr.unwrap().as_ref(),
            &ExprKind::Atom(Arc::new(Atom::Number(RLispNumber::Int(2))))
        );
    }

    #[test]
    fn test_is_list_proper() {
        // Proper list: (1 . (2 . ()))
        let pair = super::Pair::<ExprKind> {
            car: Some(Arc::new(ExprKind::Atom(Arc::new(Atom::Number(
                RLispNumber::Int(1),
            ))))),
            cdr: Some(Arc::new(ExprKind::Pair(Arc::new(super::Pair::<ExprKind> {
                car: Some(Arc::new(ExprKind::Atom(Arc::new(Atom::Number(
                    RLispNumber::Int(2),
                ))))),
                cdr: Some(Arc::new(ExprKind::List(Arc::new(List {
                    args: super::PairList::nil(),
                    object_id: 0,
                })))),
            })))),
        };

        assert!(pair.is_list());
    }

    #[test]
    fn test_is_list_improper() {
        // Improper list (dotted pair): (1 . 2)
        let pair = super::Pair::<ExprKind> {
            car: Some(Arc::new(ExprKind::Atom(Arc::new(Atom::Number(
                RLispNumber::Int(1),
            ))))),
            cdr: Some(Arc::new(ExprKind::Atom(Arc::new(Atom::Number(
                RLispNumber::Int(2),
            ))))),
        };

        assert!(!pair.is_list());
    }

    #[test]
    fn test_display_list() {
        let pair = super::Pair::<ExprKind> {
            car: Some(Arc::new(ExprKind::Atom(Arc::new(Atom::Number(
                RLispNumber::Int(1),
            ))))),
            cdr: Some(Arc::new(ExprKind::List(Arc::new(List {
                args: super::PairList::nil(),
                object_id: 0,
            })))),
        };

        let display = format!("{}", pair);
        assert!(display.contains("1"));
        assert!(display.contains("("));
        assert!(display.contains(")"));
    }

    #[test]
    fn test_display_dotted_pair() {
        let pair = super::Pair::<ExprKind> {
            car: Some(Arc::new(ExprKind::Atom(Arc::new(Atom::Number(
                RLispNumber::Int(1),
            ))))),
            cdr: Some(Arc::new(ExprKind::Atom(Arc::new(Atom::Number(
                RLispNumber::Int(2),
            ))))),
        };

        let display = format!("{}", pair);
        assert!(display.contains("."));
        assert!(display.contains("1"));
        assert!(display.contains("2"));
    }

    #[test]
    fn test_iterator_chain() {
        // Test iterating through a chain: (1 . (2 . (3 . ())))
        let pair = super::Pair::<ExprKind> {
            car: Some(Arc::new(ExprKind::Atom(Arc::new(Atom::Number(
                RLispNumber::Int(1),
            ))))),
            cdr: Some(Arc::new(ExprKind::Pair(Arc::new(super::Pair::<ExprKind> {
                car: Some(Arc::new(ExprKind::Atom(Arc::new(Atom::Number(
                    RLispNumber::Int(2),
                ))))),
                cdr: Some(Arc::new(ExprKind::Pair(Arc::new(super::Pair::<ExprKind> {
                    car: Some(Arc::new(ExprKind::Atom(Arc::new(Atom::Number(
                        RLispNumber::Int(3),
                    ))))),
                    cdr: Some(Arc::new(ExprKind::List(Arc::new(List {
                        args: super::PairList::nil(),
                        object_id: 0,
                    })))),
                })))),
            })))),
        };

        let collected: Vec<_> = pair.into_iter().collect();
        assert_eq!(collected.len(), 3);
    }
}
