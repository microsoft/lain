mod ctxt;
mod symbol;
pub mod ast;
pub mod attr;

pub use self::ctxt::Ctxt;

pub enum Derive {
    NewFuzzed,
    Mutate,
    BinarySerialize,
}