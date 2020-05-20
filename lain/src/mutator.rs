use rand::seq::SliceRandom;
use rand::Rng;

use crate::rand::distributions::uniform::{SampleBorrow, SampleUniform};
use crate::traits::*;
use crate::types::*;
use num::{Bounded, NumCast};
use num_traits::{WrappingAdd, WrappingSub};

use crate::lain_derive::NewFuzzed;

use std::ops::{Add, BitXor, Div, Mul, Sub};

#[cfg(feature = "serde_support")]
use serde::{Deserialize, Serialize};

// set these to 0 to disable
pub const CHANCE_TO_REPEAT_ARRAY_VALUE: f64 = 0.01;
pub const CHANCE_TO_PICK_INVALID_ENUM: f64 = 0.01;
pub const CHANCE_TO_IGNORE_MIN_MAX: f64 = 0.01;
pub const CHANCE_TO_IGNORE_POST_MUTATION: f64 = 0.05;

#[repr(u8)]
#[derive(Debug, Copy, Clone, NewFuzzed)]
enum MutatorOperation {
    BitFlip,

    Flip,

    Arithmetic,
}

#[derive(PartialEq, Clone, Debug)]
enum MutatorFlags {
    FuzzUpToNFields(usize),
    ShouldAlwaysPerformPostMutation(bool),
    AllChancesSucceedOrFail(bool),
}

/// Represents the mode of the mutator
#[derive(PartialEq, Clone, Copy, Debug)]
#[cfg_attr(feature = "serde_support", derive(Serialize, Deserialize))]
pub enum MutatorMode {
    /// Performs a linear bit flip from index 0 up to the max bit index, flipping `current_idx` number of bits
    WalkingBitFlip { bits: u8, current_idx: u8 },
    /// Selects interesting values for the current data type
    InterestingValues { current_idx: u8 },
    /// All-out random mutation
    Havoc,
}

/// Represents the state of the current corpus item being fuzzed.
#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde_support", derive(Serialize, Deserialize))]
pub struct CorpusFuzzingState {
    fields_fuzzed: usize,
    mode: MutatorMode,
    targeted_field_idx: usize,
    pub target_total_fields: usize,
    target_total_passes: usize,
    finished_iteration: bool,
}

impl Default for CorpusFuzzingState {
    fn default() -> Self {
        CorpusFuzzingState {
            fields_fuzzed: 0,
            mode: MutatorMode::Havoc,
            targeted_field_idx: 0,
            target_total_fields: 0,
            target_total_passes: 0,
            finished_iteration: false,
        }
    }
}

impl CorpusFuzzingState {
    pub fn reset(&mut self) {
        self.fields_fuzzed = 0;
        self.target_total_passes = 0;
        self.targeted_field_idx = 0;
        self.target_total_fields = 0;
        self.mode = MutatorMode::Havoc;
        self.finished_iteration = false;
    }
}

/// Object which provides helper routines for mutating data structures and RNG management.
#[derive(Debug)]
pub struct Mutator<R: Rng> {
    pub rng: R,
    flags: Vec<MutatorFlags>,
    corpus_state: CorpusFuzzingState,
}

impl<R: Rng> Mutator<R> {
    pub fn new(rng: R) -> Mutator<R> {
        Mutator {
            rng,
            flags: Vec::new(),
            corpus_state: CorpusFuzzingState::default(),
        }
    }

    pub fn set_mode(&mut self, mode: MutatorMode) {
        self.corpus_state.mode = mode;
    }

    pub fn mode(&self) -> MutatorMode {
        self.corpus_state.mode
    }

    pub fn get_corpus_state(&self) -> CorpusFuzzingState {
        self.corpus_state.clone()
    }

    pub fn set_corpus_state(&mut self, state: CorpusFuzzingState) {
        self.corpus_state = state;
    }

    /// Generates a random choice of the given type
    pub fn gen<T: 'static>(&mut self) -> T
    where
        T: NewFuzzed,
    {
        T::new_fuzzed(self, None)
    }

    /// TODO: Change function name. Mutates `mn` while taking into consideration the current mutator mode.
    ///
    pub fn mutate_from_mutation_mode<T>(&mut self, mn: &mut T)
    where
        T: BitXor<Output = T>
            + Add<Output = T>
            + Sub<Output = T>
            + WrappingAdd<Output = T>
            + WrappingSub<Output = T>
            + NumCast
            + Bounded
            + Copy
            + DangerousNumber<T>
            + std::fmt::Display,
    {
        // info!("{:?}", self.mode());
        // info!("num is: {}", mn);
        // info!("{}, {}, {}, {}", self.corpus_state.fields_fuzzed, self.corpus_state.targeted_field_idx, self.corpus_state.target_total_fields, self.corpus_state.target_total_passes);
        self.corpus_state.fields_fuzzed += 1;

        if self.mode() != MutatorMode::Havoc && self.corpus_state.finished_iteration
            || self.corpus_state.targeted_field_idx != self.corpus_state.fields_fuzzed - 1
        {
            return;
        }
        //println!("should be changing mode");

        match self.mode() {
            MutatorMode::WalkingBitFlip { bits, current_idx } => {
                for i in current_idx..current_idx + bits {
                    *mn = *mn ^ num::cast(1u64 << i).unwrap();
                }
            }
            MutatorMode::InterestingValues { current_idx } => {
                *mn = T::dangerous_number_at_index(current_idx as usize);
            }
            // Do nothing for havoc mode -- we let the individual mutators handle that
            MutatorMode::Havoc => {
                self.mutate(mn);
            }
        }

        self.corpus_state.finished_iteration = true;
        self.next_mode::<T>();
    }

    /// Manages the mutator mode state machine.
    ///
    /// This will basically:
    ///
    /// - Swap between the different [MutatorMode]s. This will transition from walking bit flips to dangerous numbers, to havoc mode.
    /// - For each mode, determine if there are any any other substates to exhaust (e.g. more bits to flip, more dangerous numbers to select), and update
    /// the state accordingly for the next iteration. If no other substates exist, the hard [MutatorMode] state will move to the next enum variant. Before reaching
    /// the [MutatorMode::Havoc] state, each subsequent mode will check if the last field has been mutated yet. If not, the state will reset to [MutatorMode::WalkingBitFlip]
    /// and adjust the current member index being fuzzed.
    /// - Once all members have been fuzzed in all [MutatorMode]s, the mode is set to [MutatorMode::Havoc].
    pub fn next_mode<T: Bounded + NumCast + DangerousNumber<T>>(&mut self) {
        let num_bits = (std::mem::size_of::<T>() * 8) as u8;
        //println!("previous: {:?}", self.mode());
        //println!("num bits: {}", num_bits);
        match self.mode() {
            MutatorMode::WalkingBitFlip { bits, current_idx } => {
                // current_idx + bits + 1 == num_bits
                if bits == num_bits {
                    // if we are at the max bits, move on to the next state
                    self.corpus_state.mode = MutatorMode::InterestingValues { current_idx: 0 };
                } else if current_idx + bits == num_bits {
                    self.corpus_state.mode = MutatorMode::WalkingBitFlip {
                        bits: bits + 1,
                        current_idx: 0,
                    };
                } else {
                    self.corpus_state.mode = MutatorMode::WalkingBitFlip {
                        bits,
                        current_idx: current_idx + 1,
                    };
                }
            }
            MutatorMode::InterestingValues { current_idx } => {
                if (current_idx as usize) + 1 < T::dangerous_numbers_len() {
                    self.corpus_state.mode = MutatorMode::InterestingValues {
                        current_idx: current_idx + 1,
                    };
                } else {
                    self.corpus_state.targeted_field_idx += 1;
                    if self.corpus_state.targeted_field_idx == self.corpus_state.target_total_fields
                    {
                        self.corpus_state.mode = MutatorMode::Havoc;
                    } else {
                        self.corpus_state.mode = MutatorMode::WalkingBitFlip {
                            bits: 1,
                            current_idx: 0,
                        };
                    }
                }
            }
            MutatorMode::Havoc => {
                // stay in havoc mode since we've exhausted all other states
            }
        }

        //println!("new: {:?}", self.mode());
    }

    /// Mutates a number after randomly selecting a mutation strategy (see [MutatorOperation] for a list of strategies)
    /// If a min/max is specified then a new number in this range is chosen instead of performing
    /// a bit/arithmetic mutation
    pub fn mutate<T>(&mut self, num: &mut T)
    where
        T: BitXor<Output = T>
            + Add<Output = T>
            + Sub<Output = T>
            + NumCast
            + Bounded
            + Copy
            + WrappingAdd<Output = T>
            + WrappingSub<Output = T>,
    {
        if self.mode() != MutatorMode::Havoc {
            panic!("Mutate called in non-havoc mode");
        }

        // dirty but needs to be done so we can call self.gen_chance_ignore_flags
        let flags = self.flags.clone();
        for flag in flags {
            if let MutatorFlags::FuzzUpToNFields(num_fields) = flag {
                if self.corpus_state.fields_fuzzed == num_fields
                    || !self.gen_chance_ignore_flags(50.0)
                {
                    return;
                } else {
                    self.corpus_state.fields_fuzzed += 1;
                }
            }
        }

        let operation = MutatorOperation::new_fuzzed(self, None);

        trace!("Operation selected: {:?}", operation);
        match operation {
            MutatorOperation::BitFlip => self.bit_flip(num),
            MutatorOperation::Flip => self.flip(num),
            MutatorOperation::Arithmetic => self.arithmetic(num),
        }
    }

    /// Flip a single bit in the given number.
    fn bit_flip<T>(&mut self, num: &mut T)
    where
        T: BitXor<Output = T> + Add<Output = T> + Sub<Output = T> + NumCast + Copy,
    {
        let num_bits = (std::mem::size_of::<T>() * 8) as u8;
        let idx: u8 = self.rng.gen_range(0, num_bits);

        trace!("xoring bit {}", idx);

        *num = (*num) ^ num::cast(1u64 << idx).unwrap();
    }

    /// Flip more than 1 bit in this number. This is a flip potentially up to
    /// the max bits in the number
    fn flip<T>(&mut self, num: &mut T)
    where
        T: BitXor<Output = T> + Add<Output = T> + Sub<Output = T> + NumCast + Copy,
    {
        let num_bits = (std::mem::size_of::<T>() * 8) as u8;
        let bits_to_flip = self.rng.gen_range(1, num_bits + 1) as usize;

        // 64 is chosen here as it's the the max primitive size (in bits) that we support
        // we choose to do this approach over a vec to avoid an allocation
        assert!(num_bits <= 64);
        let mut potential_bit_indices = [0u8; 64];
        for i in 0..num_bits {
            potential_bit_indices[i as usize] = i;
        }

        trace!("flipping {} bits", bits_to_flip);
        let (bit_indices, _) = potential_bit_indices[0..num_bits as usize]
            .partial_shuffle(&mut self.rng, num_bits as usize);

        for idx in bit_indices {
            *num = (*num) ^ num::cast(1u64 << *idx).unwrap()
        }
    }

    /// Perform a simple arithmetic operation on the number (+ or -)
    fn arithmetic<T>(&mut self, num: &mut T)
    where
        T: Add<Output = T>
            + Sub<Output = T>
            + NumCast
            + Copy
            + WrappingAdd<Output = T>
            + WrappingSub<Output = T>,
    {
        let added_num: i64 = self.rng.gen_range(1, 0x10);

        if self.rng.gen_range(0, 2) == 0 {
            trace!("adding {}", added_num);
            *num = num.wrapping_add(&num::cast(added_num).unwrap());
        } else {
            trace!("subtracting {}", added_num);
            *num = num.wrapping_sub(&num::cast(added_num).unwrap());
        }
    }

    /// Generates a number in the range from [min, max) (**note**: non-inclusive). Panics if min >= max.
    pub fn gen_range<T, B1>(&mut self, min: B1, max: B1) -> T
    where
        T: SampleUniform + std::fmt::Display,
        B1: SampleBorrow<T>
            + std::fmt::Display
            + Add
            + Mul
            + NumCast
            + Sub
            + PartialEq
            + PartialOrd,
    {
        if min >= max {
            panic!("cannot gen number where min ({}) >= max ({})", min, max);
        }
        trace!("generating number between {} and {}", &min, &max);
        let num = self.rng.gen_range(min, max);
        trace!("got {}", num);

        num
    }

    /// Generates a number weighted to one end of the interval
    pub fn gen_weighted_range<T, B1>(&mut self, min: B1, max: B1, weighted: Weighted) -> T
    where
        T: SampleUniform + std::fmt::Display + NumCast,
        B1: SampleBorrow<T>
            + std::fmt::Display
            + std::fmt::Debug
            + Add<Output = B1>
            + Mul<Output = B1>
            + NumCast
            + Sub<Output = B1>
            + PartialEq
            + PartialOrd
            + Copy
            + Div<Output = B1>,
    {
        use crate::rand::distributions::{Distribution, WeightedIndex};

        if weighted == Weighted::None {
            return self.gen_range(min, max);
        }

        // weighted numbers are done in a pretty dumb way, but any other way is difficult.
        // the solution is to basically subdivide the range into thirds:
        // 1. The range we're weighted towards with a 70% probability
        // 2. The "midrange" with a 20% probability
        // 3. The opposite end with what should be a 10% probability

        trace!(
            "generating weighted number between {} and {} with weight towards {:?}",
            &min,
            &max,
            weighted
        );

        let range = (max - min) + B1::from(1u8).unwrap();
        let one_third_of_range: B1 = range / B1::from(3u8).unwrap();

        let zero = B1::from(0u8).unwrap();

        let mut slices = [
            ((zero.clone(), zero.clone()), 0u8),
            ((zero.clone(), zero.clone()), 0u8),
            ((zero.clone(), zero.clone()), 0u8),
        ];

        for i in 0..3 {
            let slice_index = B1::from(i).unwrap();
            let min = min + (slice_index * one_third_of_range);
            let max = min + one_third_of_range;

            slices[i as usize] = ((min, max), 0u8);
        }

        // set up the mid range
        // these assignments here represent the weight that each range should get
        (slices[1].1) = 2;

        if weighted == Weighted::Min {
            (slices[0].1) = 7;
            (slices[2].1) = 1;
        } else {
            (slices[0].1) = 1;
            (slices[2].1) = 7;
        }

        // fixup the upper bound which may currently be wrong as a result of integer/floating point math
        // to ensure that we are truly within the user requested min/max
        (slices[2].0).1 = max;

        let dist = WeightedIndex::new(slices.iter().map(|item| item.1)).unwrap();

        let subslice_index = dist.sample(&mut self.rng);
        trace!("got {} subslice index", subslice_index);

        let bounds = slices[subslice_index].0;
        trace!("subslice has bounds {:?}", bounds);

        let num = self.rng.gen_range(bounds.0, bounds.1);

        trace!("got {}", num);

        num
    }

    /// Generates the chance to mutate a field. This will always return `true` if the current mode is
    /// [MutatorMode::Havoc].
    pub fn gen_chance_to_mutate_field(&mut self, chance_percentage: f64) -> bool {
        self.mode() != MutatorMode::Havoc || !self.gen_chance(chance_percentage)
    }

    /// Helper function for quitting the recursive mutation early if the target field has already
    /// been mutated.
    pub fn should_early_bail_mutation(&self) -> bool {
        self.mode() != MutatorMode::Havoc
            && self.corpus_state.finished_iteration
            && self.corpus_state.target_total_passes > 0
    }

    /// Returns a boolean value indicating whether or not the chance event occurred
    pub fn gen_chance(&mut self, chance_percentage: f64) -> bool {
        if chance_percentage <= 0.0 {
            return false;
        }

        if chance_percentage >= 100.0 {
            return true;
        }

        for flag in self.flags.iter() {
            if let MutatorFlags::AllChancesSucceedOrFail(should_succeed) = flag {
                return *should_succeed;
            }
        }

        self.gen_chance_ignore_flags(chance_percentage)
    }

    /// Different implementation of gen_chance that ignores the current flags
    fn gen_chance_ignore_flags(&mut self, chance_percentage: f64) -> bool {
        self.rng.gen_bool(chance_percentage)
    }

    /// Returns a boolean indicating whether or not post mutation steps should be taken
    pub fn should_fixup(&mut self) -> bool {
        self.mode() == MutatorMode::Havoc && !self.gen_chance(CHANCE_TO_IGNORE_POST_MUTATION)
        // for flag in self.flags.iter() {
        //     if let MutatorFlags::ShouldAlwaysPerformPostMutation(should_perform) = flag {
        //         return *should_perform;
        //     }
        // }

        // !self.gen_chance(CHANCE_TO_IGNORE_POST_MUTATION)
    }

    /// Client code should call this to signal to the mutator that a new fuzzer iteration is beginning
    /// and that the mutator should reset internal state.
    pub fn begin_new_iteration(&mut self) {
        let mut set_flags = [false, false, false];
        self.flags.clear();
        let temp_fields_fuzzed = self.corpus_state.fields_fuzzed;
        self.corpus_state.fields_fuzzed = 0;

        if self.corpus_state.target_total_fields == 0 {
            self.corpus_state.target_total_fields = temp_fields_fuzzed;

            if self.corpus_state.targeted_field_idx > self.corpus_state.target_total_fields {
                panic!(
                    "somehow got targeted field index {} with {} total fields",
                    self.corpus_state.targeted_field_idx, self.corpus_state.target_total_fields
                );
            }
        }

        self.corpus_state.target_total_passes += 1;
        self.corpus_state.finished_iteration = false;

        if self.mode() == MutatorMode::Havoc && self.corpus_state.target_total_fields > 0 {
            // only 2 flags can be concurrently set
            for _i in 0..self.gen_range(0, 2) {
                let flag_num = self.gen_range(0, 3);
                let flag = match flag_num {
                    0 => MutatorFlags::FuzzUpToNFields(
                        self.gen_range(1, self.corpus_state.target_total_fields + 1),
                    ),
                    1 => MutatorFlags::ShouldAlwaysPerformPostMutation(self.gen_range(0, 2) != 0),
                    2 => MutatorFlags::AllChancesSucceedOrFail(self.gen_range(0, 2) != 0),
                    _ => unreachable!(),
                };

                if !set_flags[flag_num] {
                    set_flags[flag_num] = true;
                    self.flags.push(flag);
                }
            }
        }
    }

    /// Resets the corpus state and current mutation mode.
    pub fn begin_new_corpus(&mut self) {
        self.corpus_state.reset();
        self.set_mode(MutatorMode::WalkingBitFlip {
            bits: 1,
            current_idx: 0,
        });
    }
}
