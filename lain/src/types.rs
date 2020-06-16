use num_traits::Bounded;
use std::fmt::Debug;

#[cfg(feature = "serde_support")]
use serde::{Deserialize, Serialize};

/// Represents an enum that can contain unsafe values.
///
/// These are enums which may potentially be used as indices, offsets, or used in some other
/// calculation. This wrapper type exists since the Rust compiler makes strong assumptions about how
/// enums are used, and if you attempt to unsafely (either through a union or pointers) set the
/// value of an enum to an indiscriminant value, you will regularly hit issues with illegal
/// instructions being executed while in debug mode. See, Rust will emit certain LLVM IR code like
/// `unreachable;` to give LLVM certain hints. The problem is that Rust believes (and rightfully so)
/// that enums have discrete values *unless* they are programmed to contain custom discriminant
/// values. So if you have ane num like:
/// ```
/// enum MyEnum {
///     Foo = 1,
///     Bar,
///     Baz, // ...
/// }
/// ```
///
/// Rust expects in some scenarios that *all* possible values have been accounted for so the
/// following is emitted:
///
/// ```compile_fail
/// let my_enum_instance = MyEnum::Foo;
/// match my_enum_instance {
///     MyEnum::Foo | MyEnum::Bar | MyEnum::Baz => println!("All possibilities accounted for :)"), // your code
///     _ => unreachable(), // the compiler will insert this branch in some scenarios
/// }
/// ```
///
/// But what if you set the value of your instance to something other than 1, 2, or 3 via `unsafe`
/// code? That `unreachable()` block is hit *in debug builds only* and suddenly your code doesn't
/// work. In release mode, sometimes the `_` (default) path is actually used to hold the first item
/// of the enum, so your "all other values" code path *actually* represents a very real value.
///
/// **TL;DR** Rust makes too many assumptions about how enums are used to make doing unsafe things
/// with them worthwhile. This wrapper enum works around that.
#[derive(Debug, Clone, Copy)]
#[cfg_attr(feature = "serde_support", derive(Serialize, Deserialize))]
pub enum UnsafeEnum<T, I> {
    Valid(T),
    Invalid(I),
}

impl<T, I> Default for UnsafeEnum<T, I>
where
    T: Default,
{
    fn default() -> Self {
        UnsafeEnum::Valid(Default::default())
    }
}

impl<E, T> crate::traits::ToPrimitive for UnsafeEnum<E, T>
where
    E: crate::traits::ToPrimitive<Output = T>,
    T: Copy,
{
    type Output = T;

    fn to_primitive(&self) -> T {
        match self {
            UnsafeEnum::Valid(ref e) => e.to_primitive(),
            UnsafeEnum::Invalid(n) => *n,
        }
    }
}

// TODO: Clean up this string interface. This isn't the cleanest
/// Wrapper around `String` that provides mutation methods appropriate for UTF-8 encoded Strings
#[derive(Debug, Default, Clone)]
pub struct Utf8String {
    pub(crate) inner: Vec<Utf8Char>,
}

impl Utf8String {
    pub fn new(s: &str) -> Self {
        Utf8String {
            inner: s.chars().map(|c| Utf8Char(c)).collect(),
        }
    }
}

/// Wrapper around `String` that provides mutation methods appropriate for ASCII encoded Strings
#[derive(Debug, Default, Clone)]
pub struct AsciiString {
    pub(crate) inner: Vec<AsciiChar>,
}

impl AsciiString {
    pub fn new(s: &str) -> Self {
        AsciiString {
            inner: s.chars().map(|c| AsciiChar(c)).collect(),
        }
    }
}

/// Represents a UTF-8 character.
#[derive(Default, Debug, Clone)]
pub(crate) struct Utf8Char(pub(crate) char);

/// Represents an ASCII character.
#[derive(Default, Debug, Clone)]
pub(crate) struct AsciiChar(pub(crate) char);

/// Data structure holding constraints that the [NewFuzzed::new_fuzzed][lain::traits::NewFuzzed::new_fuzzed] or
/// [Mutatable::mutate][lain::traits::Mutatable::mutate] methods should try to respect.
#[derive(Debug, Default, Clone)]
pub struct Constraints<T: Bounded + Debug> {
    /// The contextual "min" bound
    pub min: Option<T>,
    /// The contextual "max" bound (**not** inclusive)
    pub max: Option<T>,
    /// Which direction to weigh the RNG towards
    pub weighted: Weighted,
    /// The space allotted for dynamically-sized objects
    pub max_size: Option<usize>,
    pub base_object_size_accounted_for: bool,
}

impl<T: Bounded + Debug> Constraints<T> {
    pub fn new() -> Constraints<T> {
        Constraints {
            min: None,
            max: None,
            weighted: Weighted::None,
            max_size: None,
            base_object_size_accounted_for: false,
        }
    }

    pub fn min<'a>(&'a mut self, min: T) -> &'a mut Constraints<T> {
        self.min = Some(min);
        self
    }

    pub fn max<'a>(&'a mut self, max: T) -> &'a mut Constraints<T> {
        self.max = Some(max);
        self
    }

    pub fn weighted<'a>(&'a mut self, weighted: Weighted) -> &'a mut Constraints<T> {
        self.weighted = weighted;
        self
    }

    pub fn max_size<'a>(&'a mut self, max_size: usize) -> &'a mut Constraints<T> {
        self.max_size = Some(max_size);
        self
    }

    pub fn account_for_base_object_size<'a, U: crate::traits::SerializedSize>(
        &'a mut self,
    ) -> &'a mut Constraints<T> {
        if !self.base_object_size_accounted_for {
            if let Some(ref mut max_size) = self.max_size {
                if U::max_default_object_size() > *max_size {
                    panic!("minimum base object size is larger than the desired maximum output size (required at least: 0x{:X}, max: 0x{:X}). Check to ensure your output buffer for this object is larger enough", U::max_default_object_size(), *max_size);
                }

                *max_size -= U::max_default_object_size();
            }

            self.base_object_size_accounted_for = true;
        }

        self
    }

    pub fn set_base_size_accounted_for<'a>(&'a mut self) -> &'a mut Constraints<T> {
        self.base_object_size_accounted_for = true;
        self
    }
}

/// Which direction to weigh ranges towards (min bound, upper bound, or none).
#[derive(Debug, PartialEq, Clone, Copy)]
pub enum Weighted {
    None,
    Min,
    Max,
}

impl Default for Weighted {
    fn default() -> Self {
        Weighted::None
    }
}
