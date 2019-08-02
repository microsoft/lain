mod ctxt;
mod attr;
mod utils;
mod symbol;
mod ast;

pub use self::ctxt::Ctxt;

pub enum Derive {
    NewFuzzed,
    Mutate,
    BinarySerialize,
}