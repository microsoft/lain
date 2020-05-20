pub mod ast;
pub mod attr;
mod ctxt;
mod symbol;

pub use self::ctxt::Ctxt;

pub enum Derive {
    NewFuzzed,
    Mutatable,
    BinarySerialize,
}
