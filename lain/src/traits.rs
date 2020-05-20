use crate::mutator::Mutator;

use crate::rand::Rng;

use crate::types::*;
use byteorder::ByteOrder;
use num_traits::Bounded;
use std::fmt::Debug;
use std::io::Write;

/// Represents a data typethat can be pushed to a byte buffer in a constant,
/// predetermined way.
pub trait BinarySerialize {
    /// Pushes all fields in `self` to a buffer
    fn binary_serialize<W: Write, E: ByteOrder>(&self, buffer: &mut W) -> usize;
}

/// A trait to represent the output size (in bytes) of an object when serialized to binary.
pub trait SerializedSize {
    /// Serialized size in bytes of this data type
    fn serialized_size(&self) -> usize;

    /// Minimum size in bytes of this data type. This is useful for determining
    /// the smallest size that a data type with a dynamic-sized member (e.g. Vec or String)
    /// may be
    fn min_nonzero_elements_size() -> usize;

    /// Maximum size in bytes of this data type with *the minimum amount of elements*. This is useful
    /// for determining the maximum size that a data type with a dynamic-sized member (e.g. Vec or String)
    /// may be within an enum with struct members.
    fn max_default_object_size() -> usize {
        Self::min_nonzero_elements_size()
    }

    /// Minimum size of the selected enum variant.
    fn min_enum_variant_size(&self) -> usize {
        Self::min_nonzero_elements_size()
    }
}

/// A data structure that can have a new instance of itself created completely randomly, with optional constraints.
pub trait NewFuzzed {
    type RangeType: Debug + Bounded + Default;

    /// Picks a random variant of `Self`
    fn new_fuzzed<R: Rng>(
        mutator: &mut Mutator<R>,
        constraints: Option<&Constraints<Self::RangeType>>,
    ) -> Self;
}

/// A data structure that can be mutated in-place from an existing data structure, possibly generated
/// by [NewFuzzed].
pub trait Mutatable {
    type RangeType: Debug + Bounded + Default;

    fn mutate<R: Rng>(
        &mut self,
        mutator: &mut Mutator<R>,
        constraints: Option<&Constraints<Self::RangeType>>,
    );
}

/// Trait used for performing fixups of a data structure when generating a new
/// struct using [NewFuzzed].
///
/// This trait is useful when you may have dependent data types, such as a "command" struct
/// that needs to correspond with an enum.
pub trait Fixup {
    fn fixup<R: Rng>(&mut self, mutator: &mut Mutator<R>);
}

impl<T> Fixup for T {
    default fn fixup<R: Rng>(&mut self, _mutator: &mut Mutator<R>) { /* nop */
    }
}

#[doc(hidden)]
pub trait DangerousNumber<T> {
    fn select_dangerous_number<R: Rng>(rng: &mut R) -> T;

    fn dangerous_number_at_index(idx: usize) -> T;

    fn dangerous_numbers_len() -> usize;
}

/// Represents a type which can be converted to a primitive type. This should be used for enums
/// so that the serializer can generically call `YourEnum::ToPrimitive()`
pub trait ToPrimitive {
    type Output;

    fn to_primitive(&self) -> Self::Output;
}

/// Trait for objects to derive in order to specify whether or not they are variable-size.
///
/// This trait does not strictly need to be implemented, however if your data structures
/// contain dynamic-size fields, the quality of fuzzing may be slightly worse. This is because
/// calling [NewFuzzed::new_fuzzed] will, if a variable-sized field is in the data structure,
/// initialize its fields in a random order. If you are working with size constraints, it may be useful
/// to `#[derive(VariableSizeObject)]` to get random field initialization.
pub trait VariableSizeObject {
    fn is_variable_size() -> bool;
}

impl<T> VariableSizeObject for T {
    default fn is_variable_size() -> bool {
        false
    }
}

impl<T> VariableSizeObject for Vec<T> {
    fn is_variable_size() -> bool {
        true
    }
}

impl VariableSizeObject for Utf8String {
    fn is_variable_size() -> bool {
        true
    }
}

impl VariableSizeObject for AsciiString {
    fn is_variable_size() -> bool {
        true
    }
}
