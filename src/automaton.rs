use std::{fmt::Display, rc::Rc};

#[derive(Debug, Hash, Eq, PartialEq)]
pub struct State {
    pub name: Rc<str>,
}

#[derive(Debug, Hash, Eq, PartialEq)]
pub enum Symbol {
    Symbol(char),
    Epsilon,
}

impl Display for Symbol {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Symbol::Symbol(c) => write!(f, "{c}"),
            Symbol::Epsilon => write!(f, "ε"),
        }
    }
}
