use crate::mutator::Mutator;

use crate::rand::seq::SliceRandom;
use crate::rand::Rng;
use crate::traits::*;
use crate::types::*;
use num_traits::Bounded;
use std::fmt::Debug;
use std::mem::MaybeUninit;
use std::{char, cmp};

impl<T> NewFuzzed for Vec<T>
where
    T: NewFuzzed + SerializedSize,
{
    type RangeType = usize;

    default fn new_fuzzed<R: Rng>(
        mutator: &mut Mutator<R>,
        constraints: Option<&Constraints<Self::RangeType>>,
    ) -> Vec<T> {
        const MAX_NUM_ELEMENTS: usize = 0x1000;

        let mut min: Self::RangeType;
        let mut max: Self::RangeType;
        let weight: Weighted;
        let max_size: Option<usize>;
        let mut used_size: usize = 0;
        let mut output: Vec<T>;

        trace!("Generating random Vec with constraints: {:#?}", constraints);

        // if no min/max were supplied, we'll take a conservative approach of 64 elements
        match constraints {
            Some(ref constraints) => {
                min = constraints.min.unwrap_or(0);
                max = constraints.max.unwrap_or(MAX_NUM_ELEMENTS);

                if min != max {
                    if min != 0 && mutator.gen_chance(crate::mutator::CHANCE_TO_IGNORE_MIN_MAX) {
                        min = 0;
                    }

                    if constraints.max.is_some()
                        && mutator.gen_chance(crate::mutator::CHANCE_TO_IGNORE_MIN_MAX)
                    {
                        // we just hope this doesn't overflow.
                        max = constraints.max.unwrap() * 2;
                    }
                }

                weight = constraints.weighted;

                max_size = constraints.max_size;
                if let Some(max_size) = max_size {
                    max = cmp::min(max, max_size / T::min_nonzero_elements_size());
                }
            }
            None => {
                min = 0;
                max = MAX_NUM_ELEMENTS;
                max_size = None;
                weight = Weighted::None;
            }
        }

        // If min == max, that means the user probably wants this to be exactly that many elements.
        let num_elements: usize = if min == max {
            min
        } else {
            mutator.gen_weighted_range(min, max, weight)
        };

        output = Vec::with_capacity(num_elements);

        for _i in 0..num_elements {
            let element = if let Some(ref max_size) = max_size {
                T::new_fuzzed(
                    mutator,
                    Some(&Constraints::new().max_size(max_size - used_size).set_base_size_accounted_for()),
                )
            } else {
                T::new_fuzzed(mutator, None)
            };

            let element_serialized_size = element.serialized_size();

            if let Some(ref max_size) = max_size {
                if used_size + element_serialized_size > *max_size {
                    return output;
                } else {
                    used_size += element_serialized_size;
                }
            }

            output.push(element);
        }

        output
    }
}

impl<T> NewFuzzed for Vec<T>
where
    T: NewFuzzed + Clone + SerializedSize,
{
    fn new_fuzzed<R: Rng>(
        mutator: &mut Mutator<R>,
        constraints: Option<&Constraints<Self::RangeType>>,
    ) -> Vec<T> {
        const MAX_NUM_ELEMENTS: usize = 0x1000;

        let mut min: Self::RangeType;
        let mut max: Self::RangeType;
        let weight: Weighted;
        let max_size: Option<usize>;
        let mut used_size: usize = 0;
        let mut output: Vec<T>;

        trace!("Generating random Vec with constraints: {:#?}", constraints);

        // if no min/max were supplied, we'll take a conservative approach of 64 elements
        match constraints {
            Some(ref constraints) => {
                min = constraints.min.unwrap_or(0);
                max = constraints.max.unwrap_or(MAX_NUM_ELEMENTS);

                if min != max {
                    if min != 0 && mutator.gen_chance(crate::mutator::CHANCE_TO_IGNORE_MIN_MAX) {
                        min = 0;
                    }

                    if constraints.max.is_some()
                        && mutator.gen_chance(crate::mutator::CHANCE_TO_IGNORE_MIN_MAX)
                    {
                        // we just hope this doesn't overflow.
                        max = constraints.max.unwrap() * 2;
                    }
                }

                weight = constraints.weighted;

                max_size = constraints.max_size;
                if let Some(max_size) = constraints.max_size {
                    max = cmp::min(max, max_size / T::min_nonzero_elements_size());
                }
            }
            None => {
                min = 0;
                max = MAX_NUM_ELEMENTS;
                max_size = None;
                weight = Weighted::None;
            }
        }

        // If min == max, that means the user probably wants this to be exactly that many elements.
        let num_elements: usize = if min == max {
            min
        } else {
            mutator.gen_weighted_range(min, max, weight)
        };

        output = Vec::with_capacity(num_elements);

        let should_reuse_array_item =
            mutator.gen_chance(crate::mutator::CHANCE_TO_REPEAT_ARRAY_VALUE);

        if should_reuse_array_item {
            let element: T = if let Some(ref max_size) = max_size {
                T::new_fuzzed(
                    mutator,
                    Some(&Constraints::new().max_size(max_size - used_size).set_base_size_accounted_for()),
                )
            } else {
                T::new_fuzzed(mutator, None)
            };

            let element_serialized_size = element.serialized_size();

            for _i in 0..num_elements {
                if let Some(ref max_size) = max_size {
                    if used_size + element_serialized_size > *max_size {
                        return output;
                    } else {
                        used_size += element_serialized_size;
                    }
                }

                output.push(element.clone());
            }
        } else {
            for _i in 0..num_elements {
                let element: T = if let Some(ref max_size) = max_size {
                    T::new_fuzzed(
                        mutator,
                        Some(&Constraints::new().max_size(max_size - used_size)),
                    )
                } else {
                    T::new_fuzzed(mutator, None)
                };

                let element_serialized_size = element.serialized_size();

                if let Some(ref max_size) = max_size {
                    if used_size + element_serialized_size > *max_size {
                        return output;
                    } else {
                        used_size += element_serialized_size;
                    }
                }

                output.push(element);
            }
        }

        output
    }
}

// TODO: Uncomment once const generics are more stable
// impl<T, const SIZE: usize> NewFuzzed for [T; SIZE]
// where T: NewFuzzed + Clone {
//     type RangeType = usize;

//     fn new_fuzzed<R: Rng>(mutator: &mut Mutator<R>, constraints: Option<&Constraints<Self::RangeType>>) -> [T; SIZE] {
//         if constraints.is_some() {
//             warn!("Constraints passed to new_fuzzed on fixed-size array do nothing");
//         }

//         let mut output: MaybeUninit<[T; SIZE]> = MaybeUninit::uninit();
//         let arr_ptr = output.as_mut_ptr() as *mut T;

//         let mut idx = 0;
//         let mut element: T = T::new_fuzzed(mutator, None);
//         while idx < SIZE {
//             arr_ptr.add(idx).write(element.clone());

//             idx += 1;
//             if SIZE - idx > 0 {
//                 if mutator.gen_chance(crate::mutator::CHANCE_TO_REPEAT_ARRAY_VALUE) {
//                     let repeat_end_idx = mutator.gen_range(idx, SIZE);
//                     while idx < repeat_end_idx {
//                         arr_ptr.add(idx).write(element.clone());
//                         idx += 1;
//                     }

//                     if SIZE - idx > 0 {
//                         element = T::new_fuzzed(mutator, None);
//                     }
//                 } else {
//                     element = T::new_fuzzed(mutator, None);
//                 }
//             }
//         }

//         unsafe { output.assume_init() }
//     }
// }

impl<T, I> NewFuzzed for UnsafeEnum<T, I>
where
    T: NewFuzzed,
    I: NewFuzzed<RangeType = I> + Bounded + Debug + Default,
{
    type RangeType = I;

    fn new_fuzzed<R: Rng>(
        mutator: &mut Mutator<R>,
        constraints: Option<&Constraints<Self::RangeType>>,
    ) -> Self {
        trace!(
            "Generating random UnsafeEnum with constraints: {:#?}",
            constraints
        );

        if mutator.gen_chance(crate::mutator::CHANCE_TO_PICK_INVALID_ENUM) {
            UnsafeEnum::Invalid(I::new_fuzzed(mutator, constraints))
        } else {
            // TODO/BUG: We should be passing on the constraints, but all
            // objects are generated with RangeType = u8, which causes
            // complications when I is not a u8...
            UnsafeEnum::Valid(T::new_fuzzed(mutator, None))
        }
    }
}

impl NewFuzzed for Utf8String {
    type RangeType = usize;

    fn new_fuzzed<R: Rng>(
        mutator: &mut Mutator<R>,
        constraints: Option<&Constraints<Self::RangeType>>,
    ) -> Self {
        let min: Self::RangeType;
        let max: Self::RangeType;
        let weight: Weighted;
        let mut output: Utf8String;

        trace!(
            "Generating random UtfString with constraints: {:#?}",
            constraints
        );

        // if no min/max were supplied, we'll take a conservative approach
        match constraints {
            Some(ref constraints) => {
                min = constraints.min.unwrap_or(0);
                max = constraints.max.unwrap_or(256);
                weight = constraints.weighted;
            }
            None => {
                min = 0;
                max = 256;
                weight = Weighted::None;
            }
        }

        let string_length = mutator.gen_weighted_range(min, max, weight);

        output = Utf8String {
            inner: Vec::with_capacity(string_length),
        };

        let mut idx = 0;
        let mut chr = Utf8Char::new_fuzzed(mutator, None);

        while idx < string_length {
            output.inner.push(chr.clone());

            idx += 1;
            if string_length - idx > 0 {
                if mutator.gen_chance(crate::mutator::CHANCE_TO_REPEAT_ARRAY_VALUE) {
                    let repeat_end_idx = mutator.gen_range(idx, string_length);
                    while idx < repeat_end_idx {
                        output.inner.push(chr.clone());
                        idx += 1;
                    }
                    if string_length - idx > 0 {
                        chr = Utf8Char::new_fuzzed(mutator, None);
                    }
                } else {
                    chr = Utf8Char::new_fuzzed(mutator, None);
                }
            }
        }

        output
    }
}

impl NewFuzzed for AsciiString {
    type RangeType = usize;

    fn new_fuzzed<R: Rng>(
        mutator: &mut Mutator<R>,
        constraints: Option<&Constraints<Self::RangeType>>,
    ) -> Self {
        let min: Self::RangeType;
        let max: Self::RangeType;
        let weight: Weighted;
        let mut output: AsciiString;

        trace!(
            "Generating random AsciiString with constraints: {:#?}",
            constraints
        );

        // if no min/max were supplied, we'll take a conservative approach
        match constraints {
            Some(ref constraints) => {
                min = constraints.min.unwrap_or(0);
                max = constraints.max.unwrap_or(256);
                weight = constraints.weighted;
            }
            None => {
                min = 0;
                max = 256;
                weight = Weighted::None;
            }
        }

        let string_length = mutator.gen_weighted_range(min, max, weight);

        output = AsciiString {
            inner: Vec::with_capacity(string_length),
        };

        let mut idx = 0;
        let mut chr = AsciiChar::new_fuzzed(mutator, None);

        while idx < string_length {
            output.inner.push(chr.clone());

            idx += 1;
            if string_length - idx > 0 {
                if mutator.gen_chance(crate::mutator::CHANCE_TO_REPEAT_ARRAY_VALUE) {
                    let repeat_end_idx = mutator.gen_range(idx, string_length);
                    while idx < repeat_end_idx {
                        output.inner.push(chr.clone());
                        idx += 1;
                    }
                    if string_length - idx > 0 {
                        chr = AsciiChar::new_fuzzed(mutator, None);
                    }
                } else {
                    chr = AsciiChar::new_fuzzed(mutator, None);
                }
            }
        }

        output
    }
}

impl NewFuzzed for Utf8Char {
    type RangeType = u32;

    fn new_fuzzed<R: crate::rand::Rng>(
        mutator: &mut crate::mutator::Mutator<R>,
        _constraints: Option<&Constraints<Self::RangeType>>,
    ) -> Self {
        trace!("generating random UTF8 char");

        // This implementation is taken almost verbatim from burntsushi's
        // quickcheck library. See this link for the original implementation:
        // https://github.com/BurntSushi/quickcheck/blob/b3e50a5e7c85e19538cf8612d9fd6da32c588930/src/arbitrary.rs#L573-L637
        //
        // I like his logic for mode generation, so that's kept as well

        let mode_chance = mutator.gen_range(0, 100);
        match mode_chance {
            0..=49 => Utf8Char(mutator.gen_range(0, 0xB0) as u8 as char),
            50..=59 => {
                loop {
                    if let Some(c) = char::from_u32(mutator.gen_range(0, 0x10000)) {
                        return Utf8Char(c);
                    }

                    // keep looping if we got an invalid char. this will
                    // ignore surrogate pairs
                }
            }
            60..=84 => {
                // Characters often used in programming languages
                let c = [
                    ' ', ' ', ' ', '\t', '\n', '~', '`', '!', '@', '#', '$', '%', '^', '&', '*',
                    '(', ')', '_', '-', '=', '+', '[', ']', '{', '}', ':', ';', '\'', '"', '\\',
                    '|', ',', '<', '>', '.', '/', '?', '0', '1', '2', '3', '4', '5', '6', '7', '8',
                    '9',
                ]
                .choose(&mut mutator.rng)
                .unwrap()
                .to_owned();

                Utf8Char(c)
            }
            85..=89 => {
                // Tricky Unicode, part 1
                let c = [
                    '\u{0149}', // a deprecated character
                    '\u{fff0}', // some of "Other, format" category:
                    '\u{fff1}',
                    '\u{fff2}',
                    '\u{fff3}',
                    '\u{fff4}',
                    '\u{fff5}',
                    '\u{fff6}',
                    '\u{fff7}',
                    '\u{fff8}',
                    '\u{fff9}',
                    '\u{fffA}',
                    '\u{fffB}',
                    '\u{fffC}',
                    '\u{fffD}',
                    '\u{fffE}',
                    '\u{fffF}',
                    '\u{0600}',
                    '\u{0601}',
                    '\u{0602}',
                    '\u{0603}',
                    '\u{0604}',
                    '\u{0605}',
                    '\u{061C}',
                    '\u{06DD}',
                    '\u{070F}',
                    '\u{180E}',
                    '\u{110BD}',
                    '\u{1D173}',
                    '\u{e0001}', // tag
                    '\u{e0020}', //  tag space
                    '\u{e000}',
                    '\u{e001}',
                    '\u{ef8ff}', // private use
                    '\u{f0000}',
                    '\u{ffffd}',
                    '\u{ffffe}',
                    '\u{fffff}',
                    '\u{100000}',
                    '\u{10FFFD}',
                    '\u{10FFFE}',
                    '\u{10FFFF}',
                    // "Other, surrogate" characters are so that very special
                    // that they are not even allowed in safe Rust,
                    //so omitted here
                    '\u{3000}', // ideographic space
                    '\u{1680}',
                    // other space characters are already covered by two next
                    // branches
                ]
                .choose(&mut mutator.rng)
                .unwrap()
                .to_owned();

                Utf8Char(c)
            }
            90..=94 => {
                // Tricky unicode, part 2
                Utf8Char(char::from_u32(mutator.gen_range(0x2000, 0x2070)).unwrap())
            }
            95..=99 => {
                // Completely arbitrary characters
                Utf8Char(mutator.gen())
            }
            _ => unreachable!(),
        }
    }
}

impl NewFuzzed for AsciiChar {
    type RangeType = u8;

    fn new_fuzzed<R: crate::rand::Rng>(
        mutator: &mut crate::mutator::Mutator<R>,
        constraints: Option<&Constraints<Self::RangeType>>,
    ) -> Self {
        trace!("generating random ASCII char");
        let min: Self::RangeType;
        let max: Self::RangeType;
        let weight: Weighted;

        // if no min/max were supplied, we'll take a conservative approach of 64 elements
        match constraints {
            Some(ref constraints) => {
                min = constraints.min.unwrap_or(0);
                max = constraints.max.unwrap_or(0x80);
                weight = constraints.weighted;
            }
            None => {
                // If no constraints were provided, we'll use logic similar to Utf8Char above and
                // potentially generate special classes of chars

                // even though we could use gen_chance() here, let's not in case we want
                // to add more special classes
                let mode_chance = mutator.gen_range(0, 100);
                match mode_chance {
                    0..=49 => {
                        // Just generate a random char
                        return AsciiChar(mutator.gen_range(0, 0x80) as u8 as char);
                    }
                    50..=99 => {
                        // Characters often used in programming languages
                        let c = [
                            ' ', ' ', ' ', '\t', '\n', '~', '`', '!', '@', '#', '$', '%', '^', '&',
                            '*', '(', ')', '_', '-', '=', '+', '[', ']', '{', '}', ':', ';', '\'',
                            '"', '\\', '|', ',', '<', '>', '.', '/', '?', '0', '1', '2', '3', '4',
                            '5', '6', '7', '8', '9',
                        ]
                        .choose(&mut mutator.rng)
                        .unwrap()
                        .to_owned();

                        return AsciiChar(c);
                    }
                    _ => unreachable!(),
                }
            }
        }

        AsciiChar(
            std::char::from_u32(mutator.gen_weighted_range(min as u32, max as u32, weight))
                .expect("Invalid codepoint generated for AsciiChar"),
        )
    }
}

impl NewFuzzed for char {
    type RangeType = u32;

    fn new_fuzzed<R: crate::rand::Rng>(
        mutator: &mut crate::mutator::Mutator<R>,
        constraints: Option<&Constraints<Self::RangeType>>,
    ) -> Self {
        Utf8Char::new_fuzzed(mutator, constraints).0
    }
}

impl NewFuzzed for bool {
    type RangeType = u8;

    fn new_fuzzed<R: crate::rand::Rng>(
        mutator: &mut crate::mutator::Mutator<R>,
        _constraints: Option<&Constraints<Self::RangeType>>,
    ) -> Self {
        trace!("generating random bool");

        mutator.gen_range(0u8, 2u8) != 0
    }
}

macro_rules! impl_new_fuzzed {
    ( $($name:ident),* ) => {
        $(
            impl NewFuzzed for $name {
                type RangeType = $name;

                fn new_fuzzed<R: Rng>(mutator: &mut Mutator<R>, constraints: Option<&Constraints<Self::RangeType>>) -> Self {
                    let min: Self::RangeType;
                    let max: Self::RangeType;
                    let weight: Weighted;

                    // if no min/max were supplied, we'll take a conservative approach of 64 elements
                    match constraints {
                        Some(ref constraints) => {
                            min = if let Some(ref min) = constraints.min {
                                if mutator.gen_chance(crate::mutator::CHANCE_TO_IGNORE_MIN_MAX) {
                                    $name::min_value()
                                } else {
                                    *min
                                }
                            } else {
                                $name::min_value()
                            };

                            max = if let Some(ref max) = constraints.max {
                                if mutator.gen_chance(crate::mutator::CHANCE_TO_IGNORE_MIN_MAX) {
                                    $name::max_value()
                                } else {
                                    *max
                                }
                            } else {
                                $name::max_value()
                            };

                            weight = constraints.weighted;

                            return mutator.gen_weighted_range(min, max, weight);
                        }
                        None => {
                            return mutator.rng.gen();
                        }
                    }
                }
            }
        )*
    }
}

// BUG: f32/f64 generate a number between 0/1 when no constraints are supplied,
// otherwise they generate an *integer* between min/max.
impl_new_fuzzed!(u8, i8, u16, i16, u32, i32, u64, i64, f32, f64);

impl<T> NewFuzzed for [T; 0]
where
    T: NewFuzzed + Clone,
{
    type RangeType = usize;

    fn new_fuzzed<R: Rng>(
        _mutator: &mut Mutator<R>,
        _constraints: Option<&Constraints<Self::RangeType>>,
    ) -> [T; 0] {
        // no-op
        []
    }
}

macro_rules! impl_new_fuzzed_array {
    ( $($size:expr),* ) => {
        $(
            impl<T> NewFuzzed for [T; $size]
            where T: NewFuzzed + Clone + SerializedSize {
                type RangeType = usize;

                fn new_fuzzed<R: Rng>(mutator: &mut Mutator<R>, constraints: Option<&Constraints<Self::RangeType>>) -> [T; $size] {
                    let mut max_size: Option<usize> = None;

                    if let Some(ref constraints) = constraints {
                       if let Some(temp_max_size) = constraints.max_size {
                            if T::min_nonzero_elements_size() * $size  > temp_max_size {
                                warn!("max size provided to array is smaller than the min size of array");
                            }

                            max_size = Some(temp_max_size / $size)
                        }
                    }

                    let mut output: MaybeUninit<[T; $size]> = MaybeUninit::uninit();
                    let arr_ptr = output.as_mut_ptr() as *mut T;

                    let mut idx = 0;
                    let mut element: T = if let Some(max_size) = max_size {
                        T::new_fuzzed(mutator, Some(&Constraints::new().max_size(max_size)))
                    } else {
                        T::new_fuzzed(mutator, None)
                    };

                    while idx < $size {
                        unsafe {
                            arr_ptr.add(idx).write(element.clone());
                        }

                        idx += 1;
                        if $size - idx > 0 {
                            if mutator.gen_chance(crate::mutator::CHANCE_TO_REPEAT_ARRAY_VALUE) {
                                let repeat_end_idx = mutator.gen_range(idx, $size);
                                while idx < repeat_end_idx {
                                    unsafe {
                                        arr_ptr.add(idx).write(element.clone());
                                    }
                                    idx += 1;
                                }

                                if $size - idx > 0 {
                                    element = if let Some(max_size) = max_size {
                                        T::new_fuzzed(mutator, Some(&Constraints::new().max_size(max_size)))
                                    } else {
                                        T::new_fuzzed(mutator, None)
                                    };
                                }
                            } else {
                                element = if let Some(max_size) = max_size {
                                    T::new_fuzzed(mutator, Some(&Constraints::new().max_size(max_size)))
                                } else {
                                    T::new_fuzzed(mutator, None)
                                };
                            }
                        }
                    }

                    unsafe { output.assume_init() }
                }
            }

            impl<T> NewFuzzed for [T; $size]
            where T: NewFuzzed + SerializedSize {
                default type RangeType = usize;

                default fn new_fuzzed<R: Rng>(mutator: &mut Mutator<R>, constraints: Option<&Constraints<Self::RangeType>>) -> [T; $size] {
                    let mut max_size: Option<usize> = None;

                    if let Some(ref constraints) = constraints {
                       if let Some(temp_max_size) = constraints.max_size {
                            if T::min_nonzero_elements_size() * $size  > temp_max_size {
                                warn!("max size provided to array is smaller than the min size of array");
                            }

                            max_size = Some(temp_max_size / $size)
                        }
                    }

                    let mut output: MaybeUninit<[T; $size]> = MaybeUninit::uninit();
                    let arr_ptr = output.as_mut_ptr() as *mut T;

                    for i in 0..$size {
                        let element = if let Some(max_size) = max_size {
                            T::new_fuzzed(mutator, Some(&Constraints::new().max_size(max_size)))
                        } else {
                            T::new_fuzzed(mutator, None)
                        };
                        unsafe {
                            arr_ptr.offset(i).write(element);
                        }
                    }

                    unsafe { output.assume_init() }
                }
            }
        )*
    }
}

impl_new_fuzzed_array!(
    1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20, 21, 22, 23, 24, 25, 26,
    27, 28, 29, 30, 31, 32, 33, 34, 35, 36, 37, 38, 39, 40, 41, 42, 43, 44, 45, 46, 47, 48, 49, 50,
    51, 52, 53, 54, 55, 56, 57, 58, 59, 60
);
