use std::fmt::{self, Display};
use syn::{Ident, Path};

#[derive(Copy, Clone)]
pub struct Symbol(&'static str);

pub const LAIN: Symbol = Symbol("lain");
pub const MIN: Symbol = Symbol("min");
pub const MAX: Symbol = Symbol("max");
pub const IGNORE: Symbol = Symbol("ignore");
pub const IGNORE_CHANCE: Symbol = Symbol("ignore_chance");
pub const BITS: Symbol = Symbol("bits");
pub const BIG_ENDIAN: Symbol = Symbol("big_endian");
pub const LITTLE_ENDIAN: Symbol = Symbol("little_endian");
pub const INITIALIZER: Symbol = Symbol("initializer");
pub const SERIALIZED_SIZE: Symbol = Symbol("initializer");
pub const WEIGHT: Symbol = Symbol("weight");
pub const WEIGHT_TO: Symbol = Symbol("weight_to");

impl PartialEq<Symbol> for Ident {
    fn eq(&self, word: &Symbol) -> bool {
        self == word.0
    }
}

impl<'a> PartialEq<Symbol> for &'a Ident {
    fn eq(&self, word: &Symbol) -> bool {
        *self == word.0
    }
}

impl PartialEq<Symbol> for Path {
    fn eq(&self, word: &Symbol) -> bool {
        self.is_ident(word.0)
    }
}

impl Display for Symbol {
    fn fmt(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        formatter.write_str(self.0)
    }
}