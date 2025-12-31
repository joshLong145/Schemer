use crate::types::ExprKind;
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
        let car = self.car.clone();

        car
    }

    pub fn cdr(self) -> Option<Arc<ExprKind>> {
        let cdr = self.cdr.clone();

        cdr
    }
}

impl Iterator for Pair<ExprKind> {
    type Item = Arc<ExprKind>;

    fn next(&mut self) -> Option<Self::Item> {
        let mut curr = None;
        if let Some(car) = self.car.clone() {
            curr = Some(car);
        }
        if let Some(cdr) = self.cdr.to_owned() {
            match cdr.as_ref() {
                ExprKind::Pair(p) => {
                    self.cdr = p.cdr.clone();
                    self.car = p.car.clone();
                }
                _ => {
                    self.cdr = None;
                    self.car = None;
                }
            }
        } else {
            self.cdr = None;
            self.car = None;
        }

        curr
    }
}

impl std::fmt::Display for Pair<ExprKind> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if self.is_list() {
            write!(
                f,
                "({} {})",
                self.car.clone().unwrap().as_ref(),
                self.cdr.clone().unwrap().as_ref()
            )?;
        } else {
            write!(
                f,
                "({} . {})",
                self.car.clone().unwrap().as_ref(),
                self.cdr.clone().unwrap().as_ref()
            )?;
        }

        Ok(())
    }
}

impl Pair<ExprKind> {
    pub fn is_list(&self) -> bool {
        let mut cursor = self.clone();

        loop {
            if let Some(cdr) = cursor.cdr.to_owned() {
                match cdr.as_ref() {
                    ExprKind::Pair(p) => {
                        cursor.cdr = p.cdr.clone();
                        cursor.car = p.car.clone();
                    }
                    ExprKind::Quote(q) => match q.as_ref().clone().expr {
                        ExprKind::Pair(p) => {
                            cursor.cdr = p.cdr.clone();
                            cursor.car = p.car.clone();
                        }
                        _ => {
                            break;
                        }
                    },
                    _ => {
                        break;
                    }
                }
            } else {
                break;
            }
        }

        if let Some(cdr) = cursor.cdr {
            match cdr.as_ref() {
                ExprKind::List(l) => {
                    if l.args.len() < 1 {
                        return true;
                    } else {
                        return true;
                    }
                }
                ExprKind::Quote(q) => match q.as_ref().clone().expr {
                    ExprKind::List(l) => {
                        if l.args.len() < 1 {
                            return true;
                        } else {
                            return true;
                        }
                    }

                    _ => {
                        return false;
                    }
                },
                _ => {
                    return false;
                }
            }
        }

        false
    }
}

#[cfg(test)]
mod tests {
    use crate::types::{Atom, ExprKind, RLispNumber};
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
}
