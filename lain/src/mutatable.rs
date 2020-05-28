use crate::mutator::{Mutator};
use crate::rand::seq::index;
use crate::rand::Rng;
use crate::traits::*;
use crate::types::*;
use crate::NewFuzzed;

use num_traits::{Bounded, NumCast};
use num_traits::{WrappingAdd, WrappingSub};
use std::cmp::min;
use std::ops::BitXor;

// we'll shrink by a factor of 1/4, 1/2, 3/4, or down to [0, 8] bytes
#[derive(Copy, Clone, NewFuzzed, PartialEq)]
enum VecResizeCount {
    Quarter,
    Half,
    ThreeQuarters,
    FixedBytes,
    AllBytes,
}

#[derive(Copy, Clone, NewFuzzed)]
enum VecResizeDirection {
    FromBeginning,
    FromEnd,
}

#[derive(Copy, Clone, PartialEq, NewFuzzed)]
enum VecResizeType {
    Grow,
    Shrink,
}

/// Grows a `Vec`.
/// This will randomly select to grow by a factor of 1/4, 1/2, 3/4, or a fixed number of bytes
/// in the range of [1, 8]. Elements may be added randomly to the beginning or end of the the vec
fn grow_vec<T: NewFuzzed + SerializedSize, R: Rng>(
    vec: &mut Vec<T>,
    mutator: &mut Mutator<R>,
    mut max_size: Option<usize>,
) {
    // We need to take the current size of the vector into consideration
    if let Some(ref mut max_size) = max_size {
        // avoid derefing everywhere
        let mut max = *max_size;

        let current_size = vec.serialized_size();
        if current_size >= max {
            return;
        }

        max -= current_size;

        if max <= 1 || max < T::min_nonzero_elements_size() {
            return;
        }

        *max_size = max;
    }

    let resize_count = VecResizeCount::new_fuzzed(mutator, None);
    let mut num_elements = if vec.is_empty() {
        if let Some(ref max_size) = max_size {
            mutator.gen_range(1, (*max_size / T::min_nonzero_elements_size()) + 1)
        } else {
            mutator.gen_range(1, 9)
        }
    } else {
        match resize_count {
            VecResizeCount::Quarter => vec.len() / 4,
            VecResizeCount::Half => vec.len() / 2,
            VecResizeCount::ThreeQuarters => vec.len() - (vec.len() / 4),
            VecResizeCount::FixedBytes => mutator.gen_range(1, 9),
            VecResizeCount::AllBytes => {
                if let Some(ref max_size) = max_size {
                    mutator.gen_range(1, (*max_size / T::min_nonzero_elements_size()) + 1)
                } else {
                    mutator.gen_range(1, vec.len() + 1)
                }
            }
        }
    };

    // If we were given a size constraint, we need to respect it
    if let Some(ref mut max_size) = max_size {
        num_elements = min(num_elements, *max_size / T::min_nonzero_elements_size());
    }

    if num_elements == 0 {
        return;
    }

    match VecResizeDirection::new_fuzzed(mutator, None) {
        VecResizeDirection::FromBeginning => {
            // to avoid shifting the the entire vec on every iteration, we will
            // instead allocate a new vec, then extend it with the previous one
            let mut new_vec = Vec::with_capacity(num_elements);
            for _i in 0..num_elements {
                let constraints = max_size.and_then(|max_size| {
                    let mut c = Constraints::new();
                    c.max_size(max_size);
                    c.base_object_size_accounted_for = true;

                    Some(c)
                });

                let element = T::new_fuzzed(mutator, constraints.as_ref());
                if let Some(inner_max_size) = max_size {
                    // if this element is larger than the size we're allotted,
                    // then let's just exit
                    let element_size = element.serialized_size();
                    if element_size > inner_max_size {
                        break;
                    }

                    max_size = Some(inner_max_size - element_size);
                }

                new_vec.push(element);
            }

            new_vec.append(vec);
            *vec = new_vec
        }
        VecResizeDirection::FromEnd => {
            for _i in 0..num_elements {
                let constraints = max_size.and_then(|max_size| {
                    let mut c = Constraints::new();
                    c.max_size(max_size);
                    c.base_object_size_accounted_for = true;

                    Some(c)
                });

                let element = T::new_fuzzed(mutator, constraints.as_ref());
                if let Some(inner_max_size) = max_size {
                    // if this element is larger than the size we're allotted,
                    // then let's just exit
                    let element_size = element.serialized_size();
                    if element_size > inner_max_size {
                        break;
                    }

                    max_size = Some(inner_max_size - element_size);
                }

                vec.push(element);
            }
        }
    }
}

/// Shrinks a `Vec`.
/// This will randomly select to resize by a factor of 1/4, 1/2, 3/4, or a fixed number of bytes
/// in the range of [1, 8]. Elements may be removed randomly from the beginning or end of the the vec
fn shrink_vec<T, R: Rng>(vec: &mut Vec<T>, mutator: &mut Mutator<R>) {
    if vec.is_empty() {
        return;
    }

    let resize_count = VecResizeCount::new_fuzzed(mutator, None);
    let mut num_elements = match resize_count {
        VecResizeCount::Quarter => vec.len() / 4,
        VecResizeCount::Half => vec.len() / 2,
        VecResizeCount::ThreeQuarters => vec.len() - (vec.len() / 4),
        VecResizeCount::FixedBytes => mutator.gen_range(1, 9),
        VecResizeCount::AllBytes => vec.len(),
    };

    if num_elements == 0 {
        num_elements = mutator.gen_range(0, vec.len() + 1);
    }

    num_elements = std::cmp::min(num_elements, vec.len());

    // Special case probably isn't required here, but better to be explicit
    if num_elements == vec.len() {
        vec.drain(..);
        return;
    }

    match VecResizeDirection::new_fuzzed(mutator, None) {
        VecResizeDirection::FromBeginning => {
            vec.drain(0..num_elements);
        }
        VecResizeDirection::FromEnd => {
            vec.drain(vec.len() - num_elements..);
        }
    }
}

impl<T> Mutatable for Vec<T>
where
    T: Mutatable,
{
    default type RangeType = usize;

    default fn mutate<R: rand::Rng>(
        &mut self,
        mutator: &mut Mutator<R>,
        constraints: Option<&Constraints<Self::RangeType>>,
    ) {
        const CHANCE_TO_RESIZE_VEC: f64 = 0.01;

        // 1% chance to resize this vec
        if mutator.gen_chance(CHANCE_TO_RESIZE_VEC) {
            shrink_vec(self, mutator);
        } else {
            // Recreate the constraints so that the min/max types match
            let constraints = constraints.and_then(|c| {
                if c.max_size.is_none() {
                    None
                } else {
                    let mut new_constraints = Constraints::new();
                    new_constraints.base_object_size_accounted_for =
                        c.base_object_size_accounted_for;
                    new_constraints.max_size = new_constraints.max_size;

                    Some(new_constraints)
                }
            });

            self.as_mut_slice().mutate(mutator, constraints.as_ref());
        }
    }
}

impl<T> Mutatable for Vec<T>
where
    T: Mutatable + NewFuzzed + SerializedSize,
{
    type RangeType = usize;

    fn mutate<R: rand::Rng>(
        &mut self,
        mutator: &mut Mutator<R>,
        constraints: Option<&Constraints<Self::RangeType>>,
    ) {
        const CHANCE_TO_RESIZE_VEC: f64 = 0.01;

        if T::min_nonzero_elements_size() == 0 {
            warn!("Size of element in vec is 0... returning early");
            return;
        }

        if mutator.gen_chance(CHANCE_TO_RESIZE_VEC) {
            let resize_type = VecResizeType::new_fuzzed(mutator, None);
            if resize_type == VecResizeType::Grow {
                grow_vec(self, mutator, constraints.and_then(|c| c.max_size));
            } else {
                shrink_vec(self, mutator);
            }
        } else {
            // Recreate the constraints so that the min/max types match
            let constraints = constraints.and_then(|c| {
                if c.max_size.is_none() {
                    None
                } else {
                    let mut new_constraints = Constraints::new();
                    new_constraints.base_object_size_accounted_for =
                        c.base_object_size_accounted_for;
                    new_constraints.max_size = new_constraints.max_size;

                    Some(new_constraints)
                }
            });

            self.as_mut_slice().mutate(mutator, constraints.as_ref());
        }
    }
}

impl<T> Mutatable for [T]
where
    T: Mutatable,
{
    type RangeType = T::RangeType;

    fn mutate<R: Rng>(
        &mut self,
        mutator: &mut Mutator<R>,
        constraints: Option<&Constraints<Self::RangeType>>,
    ) {
        for item in self.iter_mut() {
            T::mutate(item, mutator, constraints);
        }
    }
}

impl Mutatable for bool {
    type RangeType = u8;

    fn mutate<R: Rng>(
        &mut self,
        mutator: &mut Mutator<R>,
        _constraints: Option<&Constraints<Self::RangeType>>,
    ) {
        *self = mutator.gen_range(0u8, 2u8) != 0;
    }
}

impl<T, I> Mutatable for UnsafeEnum<T, I>
where
    T: ToPrimitive<Output = I>,
    I: BitXor<Output = I>
        + NumCast
        + Bounded
        + Copy
        + std::fmt::Debug
        + Default
        + DangerousNumber<I>
        + std::fmt::Display
        + WrappingAdd
        + WrappingSub,
{
    type RangeType = I;

    fn mutate<R: Rng>(
        &mut self,
        mutator: &mut Mutator<R>,
        _constraints: Option<&Constraints<Self::RangeType>>,
    ) {
        if let UnsafeEnum::Valid(ref value) = *self {
            *self = UnsafeEnum::Invalid(value.to_primitive());
        }

        match *self {
            UnsafeEnum::Invalid(ref mut value) => {
                mutator.mutate(value);
            }
            _ => unreachable!(),
        }
    }
}

impl Mutatable for AsciiString {
    type RangeType = u8;

    fn mutate<R: Rng>(
        &mut self,
        mutator: &mut Mutator<R>,
        _constraints: Option<&Constraints<Self::RangeType>>,
    ) {
        trace!("performing mutation on an AsciiString");

        // TODO: Implement logic for resizing?
        let num_mutations = mutator.gen_range(1, self.inner.len());
        for idx in index::sample(&mut mutator.rng, self.inner.len(), num_mutations).iter() {
            self.inner[idx] = AsciiChar::new_fuzzed(mutator, None);
        }
    }
}

impl Mutatable for Utf8String {
    type RangeType = u8;

    fn mutate<R: Rng>(
        &mut self,
        mutator: &mut Mutator<R>,
        _constraints: Option<&Constraints<Self::RangeType>>,
    ) {
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
                type RangeType = $name;

                #[inline(always)]
                fn mutate<R: Rng>(&mut self, mutator: &mut Mutator<R>, _constraints: Option<&Constraints<Self::RangeType>>) {
                    mutator.mutate(self);
                }
            }
        )*
    }
}

impl_mutatable!(u64, u32, u16, u8);

impl Mutatable for i8 {
    type RangeType = i8;

    #[inline(always)]
    fn mutate<R: Rng>(
        &mut self,
        mutator: &mut Mutator<R>,
        _constraints: Option<&Constraints<Self::RangeType>>,
    ) {
        let mut val = *self as u8;
        mutator.mutate(&mut val);
        *self = val as i8;
    }
}

impl Mutatable for i16 {
    type RangeType = i16;

    #[inline(always)]
    fn mutate<R: Rng>(
        &mut self,
        mutator: &mut Mutator<R>,
        _constraints: Option<&Constraints<Self::RangeType>>,
    ) {
        let mut val = *self as u16;
        mutator.mutate(&mut val);
        *self = val as i16;
    }
}

impl Mutatable for i32 {
    type RangeType = i32;

    #[inline(always)]
    fn mutate<R: Rng>(
        &mut self,
        mutator: &mut Mutator<R>,
        _constraints: Option<&Constraints<Self::RangeType>>,
    ) {
        let mut val = *self as u32;
        mutator.mutate(&mut val);
        *self = val as i32;
    }
}

impl Mutatable for i64 {
    type RangeType = i64;

    #[inline(always)]
    fn mutate<R: Rng>(
        &mut self,
        mutator: &mut Mutator<R>,
        _constraints: Option<&Constraints<Self::RangeType>>,
    ) {
        let mut val = *self as u64;
        mutator.mutate(&mut val);
        *self = val as i64;
    }
}

impl<T> Mutatable for [T; 0]
where
    T: Mutatable,
{
    type RangeType = u8;

    fn mutate<R: Rng>(
        &mut self,
        _mutator: &mut Mutator<R>,
        _constraints: Option<&Constraints<Self::RangeType>>,
    ) {
        // nop
    }
}

impl Mutatable for *const std::ffi::c_void {
    type RangeType = u8;

    fn mutate<R: Rng>(
        &mut self,
        _mutator: &mut Mutator<R>,
        _constraints: Option<&Constraints<Self::RangeType>>,
    ) {
        // nop
    }
}

impl Mutatable for *mut std::ffi::c_void {
    type RangeType = u8;

    fn mutate<R: Rng>(
        &mut self,
        _mutator: &mut Mutator<R>,
        _constraints: Option<&Constraints<Self::RangeType>>,
    ) {
        // nop
    }
}

impl<T> Mutatable for Option<T>
where
    T: Mutatable + NewFuzzed,
{
    type RangeType = <T as Mutatable>::RangeType;

    fn mutate<R: Rng>(
        &mut self,
        mutator: &mut Mutator<R>,
        constraints: Option<&Constraints<Self::RangeType>>,
    ) {
        const CHANCE_TO_FLIP_OPTION_STATE: f64 = 0.01;
        match self {
            Some(inner) => {
                // small chance to make this None
                if mutator.gen_chance(CHANCE_TO_FLIP_OPTION_STATE) {
                    *self = None;
                } else {
                    inner.mutate(mutator, constraints);
                }
            }
            None => {
                if mutator.gen_chance(CHANCE_TO_FLIP_OPTION_STATE) {
                    // hack to avoid converting between constraints types even though
                    // they should be the same...
                    let mut new_item = T::new_fuzzed(mutator, None);
                    if constraints.is_some() {
                        // force the constraints if they were provided
                        new_item.mutate(mutator, constraints);
                    }

                    *self = Some(new_item);
                }
            }
        }
    }
}

macro_rules! impl_mutatable_array {
    ( $($size:expr),* ) => {
        $(
            impl<T> Mutatable for [T; $size]
            where T: Mutatable {
                type RangeType = T::RangeType;

                #[inline(always)]
                fn mutate<R: Rng>(&mut self, mutator: &mut Mutator<R>, constraints: Option<&Constraints<Self::RangeType>>) {
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
