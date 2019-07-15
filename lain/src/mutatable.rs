use crate::mutator::Mutator;
use crate::rand::seq::index;
use crate::rand::Rng;
use crate::traits::*;
use crate::types::*;

use num_traits::{Bounded, NumCast};
use num_traits::{WrappingAdd, WrappingSub};
use std::ops::BitXor;

impl<T> Mutatable for Vec<T>
where
    T: Mutatable,
{
    fn mutate<R: rand::Rng>(
        &mut self,
        mutator: &mut Mutator<R>,
        _constraints: Option<&Constraints<u8>>,
    ) {
        self.as_mut_slice().mutate(mutator, None);
    }
}

impl<T> Mutatable for [T]
where
    T: Mutatable,
{
    fn mutate<R: Rng>(&mut self, mutator: &mut Mutator<R>, _constraints: Option<&Constraints<u8>>) {
        for item in self.iter_mut() {
            T::mutate(item, mutator, None);
        }
    }
}

impl Mutatable for bool {
    fn mutate<R: Rng>(&mut self, mutator: &mut Mutator<R>, _constraints: Option<&Constraints<u8>>) {
        *self = mutator.gen_range(0u8, 2u8) != 0;
    }
}

impl<T, I> Mutatable for UnsafeEnum<T, I>
where
    T: ToPrimitive<I>,
    I: BitXor<Output = I>
        + NumCast
        + Bounded
        + Copy
        + DangerousNumber<I>
        + std::fmt::Display
        + WrappingAdd
        + WrappingSub,
{
    fn mutate<R: Rng>(&mut self, mutator: &mut Mutator<R>, _constraints: Option<&Constraints<u8>>) {
        if let UnsafeEnum::Valid(ref value) = *self {
            *self = UnsafeEnum::Invalid(value.to_primitive());
        }

        match *self {
            UnsafeEnum::Invalid(ref mut value) => {
                mutator.mutate_from_mutation_mode(value);
            }
            _ => unreachable!(),
        }
    }
}

impl Mutatable for AsciiString {
    fn mutate<R: Rng>(&mut self, mutator: &mut Mutator<R>, _constraints: Option<&Constraints<u8>>) {
        trace!("performing mutation on an AsciiString");

        // TODO: Implement logic for resizing?
        let num_mutations = mutator.gen_range(1, self.inner.len());
        for idx in index::sample(&mut mutator.rng, self.inner.len(), num_mutations).iter() {
            self.inner[idx] = AsciiChar::new_fuzzed(mutator, None);
        }
    }
}

impl Mutatable for Utf8String {
    fn mutate<R: Rng>(&mut self, mutator: &mut Mutator<R>, _constraints: Option<&Constraints<u8>>) {
        trace!("performing mutation on a Utf8String");

        // TODO: Implement logic for resizing?
        let num_mutations = mutator.gen_range(1, self.inner.len());
        for idx in index::sample(&mut mutator.rng, self.inner.len(), num_mutations).iter() {
            self.inner[idx] = Utf8Char::new_fuzzed(mutator, None);
        }
    }
}

macro_rules! impl_mutatable {
    ( $($name:ident),* ) => {
        $(
            impl Mutatable for $name {
                #[inline(always)]
                fn mutate<R: Rng>(&mut self, mutator: &mut Mutator<R>, _constraints: Option<&Constraints<u8>>) {
                    mutator.mutate_from_mutation_mode(self);
                }
            }
        )*
    }
}

impl_mutatable!(i64, u64, i32, u32, i16, u16, i8, u8);

impl<T> Mutatable for [T; 0]
where
    T: Mutatable,
{
    fn mutate<R: Rng>(
        &mut self,
        _mutator: &mut Mutator<R>,
        _constraints: Option<&Constraints<u8>>,
    ) {
        // nop
    }
}

macro_rules! impl_mutatable_array {
    ( $($size:expr),* ) => {
        $(
            impl<T> Mutatable for [T; $size]
            where T: Mutatable {
                #[inline(always)]
                fn mutate<R: Rng>(&mut self, mutator: &mut Mutator<R>, constraints: Option<&Constraints<u8>>) {
                    // Treat this as a slice
                    self[0..].mutate(mutator, constraints);
                }
            }
        )*
    }
}

impl_mutatable_array!(
    1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20, 21, 22, 23, 24, 25, 26,
    27, 28, 29, 30, 31, 32, 33, 34, 35, 36, 37, 38, 39, 40, 41, 42, 43, 44, 45, 46, 47, 48, 49, 50,
    51, 52, 53, 54, 55, 56, 57, 58, 59, 60
);
