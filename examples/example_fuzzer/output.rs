#![feature(prelude_import)]
#![no_std]
#![feature(specialization)]
#[prelude_import]
use ::std::prelude::v1::*;
#[macro_use]
extern crate std as std;
extern crate ctrlc;
extern crate lain;
use lain::driver::*;
use lain::prelude::*;
use lain::rand::Rng;
use std::io::prelude::*;
use std::net::TcpStream;
use std::sync::{Arc, RwLock};
const THREAD_COUNT: usize = 10;
struct FuzzerThreadContext {
    last_packet: Option<PacketData>,
    scratch_packet: PacketData,
    thread_packet_iterations: usize,
}
#[automatically_derived]
#[allow(unused_qualifications)]
impl ::std::default::Default for FuzzerThreadContext {
    #[inline]
    fn default() -> FuzzerThreadContext {
        FuzzerThreadContext {
            last_packet: ::std::default::Default::default(),
            scratch_packet: ::std::default::Default::default(),
            thread_packet_iterations: ::std::default::Default::default(),
        }
    }
}
struct GlobalContext {}
#[automatically_derived]
#[allow(unused_qualifications)]
impl ::std::default::Default for GlobalContext {
    #[inline]
    fn default() -> GlobalContext {
        GlobalContext {}
    }
}
struct PacketData {
    typ: UnsafeEnum<PacketType, u32>,
    offset: u64,
    length: u64,
    #[fuzzer(min = 0, max = 10)]
    data: Vec<u8>,
}
#[automatically_derived]
#[allow(unused_qualifications)]
impl ::std::fmt::Debug for PacketData {
    fn fmt(&self, f: &mut ::std::fmt::Formatter) -> ::std::fmt::Result {
        match *self {
            PacketData {
                typ: ref __self_0_0,
                offset: ref __self_0_1,
                length: ref __self_0_2,
                data: ref __self_0_3,
            } => {
                let mut debug_trait_builder = f.debug_struct("PacketData");
                let _ = debug_trait_builder.field("typ", &&(*__self_0_0));
                let _ = debug_trait_builder.field("offset", &&(*__self_0_1));
                let _ = debug_trait_builder.field("length", &&(*__self_0_2));
                let _ = debug_trait_builder.field("data", &&(*__self_0_3));
                debug_trait_builder.finish()
            }
        }
    }
}
#[automatically_derived]
#[allow(unused_qualifications)]
impl ::std::default::Default for PacketData {
    #[inline]
    fn default() -> PacketData {
        PacketData {
            typ: ::std::default::Default::default(),
            offset: ::std::default::Default::default(),
            length: ::std::default::Default::default(),
            data: ::std::default::Default::default(),
        }
    }
}
#[automatically_derived]
#[allow(unused_qualifications)]
impl ::std::clone::Clone for PacketData {
    #[inline]
    fn clone(&self) -> PacketData {
        match *self {
            PacketData {
                typ: ref __self_0_0,
                offset: ref __self_0_1,
                length: ref __self_0_2,
                data: ref __self_0_3,
            } => PacketData {
                typ: ::std::clone::Clone::clone(&(*__self_0_0)),
                offset: ::std::clone::Clone::clone(&(*__self_0_1)),
                length: ::std::clone::Clone::clone(&(*__self_0_2)),
                data: ::std::clone::Clone::clone(&(*__self_0_3)),
            },
        }
    }
}
impl ::lain::traits::PostFuzzerIteration for PacketData {
    fn on_success_for_fields(&self) {
        <UnsafeEnum<PacketType, u32>>::on_success(&self.typ);
        <u64>::on_success(&self.offset);
        <u64>::on_success(&self.length);
        <Vec<u8>>::on_success(&self.data);
    }
}
impl ::lain::traits::FixupChildren for PacketData {
    fn fixup_children<R: ::lain::rand::Rng>(&mut self, mutator: &mut Mutator<R>) {
        <UnsafeEnum<PacketType, u32>>::fixup(&mut self.typ, mutator);
        <u64>::fixup(&mut self.offset, mutator);
        <u64>::fixup(&mut self.length, mutator);
        <Vec<u8>>::fixup(&mut self.data, mutator);
    }
}
impl ::lain::traits::NewFuzzed for PacketData {
    type RangeType = u8;
    fn new_fuzzed<R: ::lain::rand::Rng>(
        mutator: &mut ::lain::mutator::Mutator<R>,
        mut constraints: Option<&::lain::types::Constraints<Self::RangeType>>,
    ) -> PacketData {
        use ::lain::rand::seq::index::sample;
        use std::any::Any;
        let mut max_size = if let Some(ref mut constraints) = constraints {
            constraints.max_size.clone()
        } else {
            None
        };
        let mut uninit_struct = std::mem::MaybeUninit::<PacketData>::uninit();
        let uninit_struct_ptr = uninit_struct.as_mut_ptr();
        let range = if Self::is_variable_size() {
            for i in sample(&mut mutator.rng, 4usize, 4usize).iter() {
                match i {
                    0usize => {
                        let constraints = if let Some(ref max_size) = max_size {
                            let mut constraints = ::lain::types::Constraints::default();
                            constraints.max_size = Some(max_size.clone());
                            Some(constraints)
                        } else {
                            None
                        };
                        let value = <UnsafeEnum<PacketType, u32>>::new_fuzzed(
                            mutator,
                            constraints.as_ref(),
                        );
                        if let Some(ref mut max_size) = max_size {
                            *max_size -= value.serialized_size();
                        }
                        let field_offset = unsafe {
                            ::field_offset::FieldOffset::<PacketData, _>::new(|x| {
                                let PacketData { ref typ, .. } = *x;
                                typ
                            })
                        }
                        .get_byte_offset() as isize;
                        unsafe {
                            let field_ptr = (uninit_struct_ptr as *mut u8).offset(field_offset)
                                as *mut UnsafeEnum<PacketType, u32>;
                            std::ptr::write(field_ptr, value);
                        }
                    }
                    1usize => {
                        let constraints = if let Some(ref max_size) = max_size {
                            let mut constraints = ::lain::types::Constraints::default();
                            constraints.max_size = Some(max_size.clone());
                            Some(constraints)
                        } else {
                            None
                        };
                        let value = <u64>::new_fuzzed(mutator, constraints.as_ref());
                        if let Some(ref mut max_size) = max_size {
                            *max_size -= value.serialized_size();
                        }
                        let field_offset = unsafe {
                            ::field_offset::FieldOffset::<PacketData, _>::new(|x| {
                                let PacketData { ref offset, .. } = *x;
                                offset
                            })
                        }
                        .get_byte_offset() as isize;
                        unsafe {
                            let field_ptr =
                                (uninit_struct_ptr as *mut u8).offset(field_offset) as *mut u64;
                            std::ptr::write(field_ptr, value);
                        }
                    }
                    2usize => {
                        let constraints = if let Some(ref max_size) = max_size {
                            let mut constraints = ::lain::types::Constraints::default();
                            constraints.max_size = Some(max_size.clone());
                            Some(constraints)
                        } else {
                            None
                        };
                        let value = <u64>::new_fuzzed(mutator, constraints.as_ref());
                        if let Some(ref mut max_size) = max_size {
                            *max_size -= value.serialized_size();
                        }
                        let field_offset = unsafe {
                            ::field_offset::FieldOffset::<PacketData, _>::new(|x| {
                                let PacketData { ref length, .. } = *x;
                                length
                            })
                        }
                        .get_byte_offset() as isize;
                        unsafe {
                            let field_ptr =
                                (uninit_struct_ptr as *mut u8).offset(field_offset) as *mut u64;
                            std::ptr::write(field_ptr, value);
                        }
                    }
                    3usize => {
                        let constraints: Option<
                            ::lain::types::Constraints<
                                <Vec<u8> as ::lain::traits::NewFuzzed>::RangeType,
                            >,
                        > = Some(Constraints {
                            min: Some(0),
                            max: Some(10),
                            weighted: ::lain::types::Weighted::None,
                            max_size: max_size.clone(),
                        });
                        let value = <Vec<u8>>::new_fuzzed(mutator, constraints.as_ref());
                        if let Some(ref mut max_size) = max_size {
                            *max_size -= value.serialized_size();
                        }
                        let field_offset = unsafe {
                            ::field_offset::FieldOffset::<PacketData, _>::new(|x| {
                                let PacketData { ref data, .. } = *x;
                                data
                            })
                        }
                        .get_byte_offset() as isize;
                        unsafe {
                            let field_ptr =
                                (uninit_struct_ptr as *mut u8).offset(field_offset) as *mut Vec<u8>;
                            std::ptr::write(field_ptr, value);
                        }
                    }
                    _ => ::std::rt::begin_panic(
                        "internal error: entered unreachable code",
                        &("example_fuzzer/src/main.rs", 31u32, 69u32),
                    ),
                }
            }
        } else {
            let constraints = if let Some(ref max_size) = max_size {
                let mut constraints = ::lain::types::Constraints::default();
                constraints.max_size = Some(max_size.clone());
                Some(constraints)
            } else {
                None
            };
            let value = <UnsafeEnum<PacketType, u32>>::new_fuzzed(mutator, constraints.as_ref());
            if let Some(ref mut max_size) = max_size {
                *max_size -= value.serialized_size();
            }
            let field_offset = unsafe {
                ::field_offset::FieldOffset::<PacketData, _>::new(|x| {
                    let PacketData { ref typ, .. } = *x;
                    typ
                })
            }
            .get_byte_offset() as isize;
            unsafe {
                let field_ptr = (uninit_struct_ptr as *mut u8).offset(field_offset)
                    as *mut UnsafeEnum<PacketType, u32>;
                std::ptr::write(field_ptr, value);
            }
            let constraints = if let Some(ref max_size) = max_size {
                let mut constraints = ::lain::types::Constraints::default();
                constraints.max_size = Some(max_size.clone());
                Some(constraints)
            } else {
                None
            };
            let value = <u64>::new_fuzzed(mutator, constraints.as_ref());
            if let Some(ref mut max_size) = max_size {
                *max_size -= value.serialized_size();
            }
            let field_offset = unsafe {
                ::field_offset::FieldOffset::<PacketData, _>::new(|x| {
                    let PacketData { ref offset, .. } = *x;
                    offset
                })
            }
            .get_byte_offset() as isize;
            unsafe {
                let field_ptr = (uninit_struct_ptr as *mut u8).offset(field_offset) as *mut u64;
                std::ptr::write(field_ptr, value);
            }
            let constraints = if let Some(ref max_size) = max_size {
                let mut constraints = ::lain::types::Constraints::default();
                constraints.max_size = Some(max_size.clone());
                Some(constraints)
            } else {
                None
            };
            let value = <u64>::new_fuzzed(mutator, constraints.as_ref());
            if let Some(ref mut max_size) = max_size {
                *max_size -= value.serialized_size();
            }
            let field_offset = unsafe {
                ::field_offset::FieldOffset::<PacketData, _>::new(|x| {
                    let PacketData { ref length, .. } = *x;
                    length
                })
            }
            .get_byte_offset() as isize;
            unsafe {
                let field_ptr = (uninit_struct_ptr as *mut u8).offset(field_offset) as *mut u64;
                std::ptr::write(field_ptr, value);
            }
            let constraints: Option<
                ::lain::types::Constraints<<Vec<u8> as ::lain::traits::NewFuzzed>::RangeType>,
            > = Some(Constraints {
                min: Some(0),
                max: Some(10),
                weighted: ::lain::types::Weighted::None,
                max_size: max_size.clone(),
            });
            let value = <Vec<u8>>::new_fuzzed(mutator, constraints.as_ref());
            if let Some(ref mut max_size) = max_size {
                *max_size -= value.serialized_size();
            }
            let field_offset = unsafe {
                ::field_offset::FieldOffset::<PacketData, _>::new(|x| {
                    let PacketData { ref data, .. } = *x;
                    data
                })
            }
            .get_byte_offset() as isize;
            unsafe {
                let field_ptr = (uninit_struct_ptr as *mut u8).offset(field_offset) as *mut Vec<u8>;
                std::ptr::write(field_ptr, value);
            }
        };
        let mut initialized_struct = unsafe { uninit_struct.assume_init() };
        if mutator.should_fixup() {
            initialized_struct.fixup(mutator);
        }
        initialized_struct
    }
}
impl ::lain::traits::Mutatable for PacketData {
    #[allow(unused)]
    fn mutate<R: ::lain::rand::Rng>(
        &mut self,
        mutator: &mut ::lain::mutator::Mutator<R>,
        constraints: Option<&Constraints<u8>>,
    ) {
        <UnsafeEnum<PacketType, u32>>::mutate(&mut self.typ, mutator, constraints);
        if mutator.should_early_bail_mutation() {
            if mutator.should_fixup() {
                <UnsafeEnum<PacketType, u32>>::fixup(&mut self.typ, mutator);
            }
            return;
        }
        <u64>::mutate(&mut self.offset, mutator, constraints);
        if mutator.should_early_bail_mutation() {
            if mutator.should_fixup() {
                <u64>::fixup(&mut self.offset, mutator);
            }
            return;
        }
        <u64>::mutate(&mut self.length, mutator, constraints);
        if mutator.should_early_bail_mutation() {
            if mutator.should_fixup() {
                <u64>::fixup(&mut self.length, mutator);
            }
            return;
        }
        <Vec<u8>>::mutate(&mut self.data, mutator, constraints);
        if mutator.should_early_bail_mutation() {
            if mutator.should_fixup() {
                <Vec<u8>>::fixup(&mut self.data, mutator);
            }
            return;
        }
        if mutator.should_fixup() {
            self.fixup(mutator);
        }
    }
}
impl ::lain::traits::VariableSizeObject for PacketData {
    fn is_variable_size() -> bool {
        false
            || <UnsafeEnum<PacketType, u32>>::is_variable_size()
            || <u64>::is_variable_size()
            || <u64>::is_variable_size()
            || <Vec<u8>>::is_variable_size()
    }
}
impl ::lain::traits::BinarySerialize for PacketData {
    fn binary_serialize<W: std::io::Write, E: ::lain::byteorder::ByteOrder>(&self, buffer: &mut W) {
        use ::lain::byteorder::{BigEndian, LittleEndian, WriteBytesExt};
        use ::lain::traits::SerializedSize;
        let mut bitfield: u64 = 0;
        self.typ.binary_serialize::<_, E>(buffer);
        self.offset.binary_serialize::<_, E>(buffer);
        self.length.binary_serialize::<_, E>(buffer);
        self.data.binary_serialize::<_, E>(buffer);
    }
}
impl ::lain::traits::SerializedSize for PacketData {
    #[inline(always)]
    fn serialized_size(&self) -> usize {
        use ::lain::traits::SerializedSize;
        {
            let lvl = ::log::Level::Debug;
            if lvl <= ::log::STATIC_MAX_LEVEL && lvl <= ::log::max_level() {
                ::log::__private_api_log(
                    ::std::fmt::Arguments::new_v1(
                        &["getting serialized size of "],
                        &match (&"PacketData",) {
                            (arg0,) => {
                                [::std::fmt::ArgumentV1::new(arg0, ::std::fmt::Display::fmt)]
                            }
                        },
                    ),
                    lvl,
                    &(
                        "example_fuzzer",
                        "example_fuzzer",
                        "example_fuzzer/src/main.rs",
                        31u32,
                    ),
                );
            }
        };
        let size = 0
            + self.typ.serialized_size()
            + std::mem::size_of::<u64>()
            + std::mem::size_of::<u64>()
            + self.data.serialized_size();
        {
            let lvl = ::log::Level::Debug;
            if lvl <= ::log::STATIC_MAX_LEVEL && lvl <= ::log::max_level() {
                ::log::__private_api_log(
                    ::std::fmt::Arguments::new_v1_formatted(
                        &["size of ", " is 0x"],
                        &match (&"PacketData", &size) {
                            (arg0, arg1) => [
                                ::std::fmt::ArgumentV1::new(arg0, ::std::fmt::Display::fmt),
                                ::std::fmt::ArgumentV1::new(arg1, ::std::fmt::UpperHex::fmt),
                            ],
                        },
                        &[
                            ::std::fmt::rt::v1::Argument {
                                position: ::std::fmt::rt::v1::Position::At(0usize),
                                format: ::std::fmt::rt::v1::FormatSpec {
                                    fill: ' ',
                                    align: ::std::fmt::rt::v1::Alignment::Unknown,
                                    flags: 0u32,
                                    precision: ::std::fmt::rt::v1::Count::Implied,
                                    width: ::std::fmt::rt::v1::Count::Implied,
                                },
                            },
                            ::std::fmt::rt::v1::Argument {
                                position: ::std::fmt::rt::v1::Position::At(1usize),
                                format: ::std::fmt::rt::v1::FormatSpec {
                                    fill: ' ',
                                    align: ::std::fmt::rt::v1::Alignment::Unknown,
                                    flags: 8u32,
                                    precision: ::std::fmt::rt::v1::Count::Implied,
                                    width: ::std::fmt::rt::v1::Count::Is(2usize),
                                },
                            },
                        ],
                    ),
                    lvl,
                    &(
                        "example_fuzzer",
                        "example_fuzzer",
                        "example_fuzzer/src/main.rs",
                        31u32,
                    ),
                );
            }
        };
        return size;
    }
    #[inline(always)]
    fn min_nonzero_elements_size() -> usize {
        0 + <UnsafeEnum<PacketType, u32>>::min_nonzero_elements_size()
            + std::mem::size_of::<u64>()
            + std::mem::size_of::<u64>()
            + <Vec<u8>>::min_nonzero_elements_size()
    }
}
impl Fixup for PacketData {
    fn fixup<R: Rng>(&mut self, mutator: &mut Mutator<R>) {
        self.length = self.data.len() as u64;
        self.fixup_children(mutator);
    }
}
#[repr(u32)]
#[rustc_copy_clone_marker]
enum PacketType {
    Read = 0x0,
    Write = 0x1,
    Reset = 0x2,
}
#[automatically_derived]
#[allow(unused_qualifications)]
impl ::std::fmt::Debug for PacketType {
    fn fmt(&self, f: &mut ::std::fmt::Formatter) -> ::std::fmt::Result {
        match (&*self,) {
            (&PacketType::Read,) => {
                let mut debug_trait_builder = f.debug_tuple("Read");
                debug_trait_builder.finish()
            }
            (&PacketType::Write,) => {
                let mut debug_trait_builder = f.debug_tuple("Write");
                debug_trait_builder.finish()
            }
            (&PacketType::Reset,) => {
                let mut debug_trait_builder = f.debug_tuple("Reset");
                debug_trait_builder.finish()
            }
        }
    }
}
#[automatically_derived]
#[allow(unused_qualifications)]
impl ::std::marker::Copy for PacketType {}
#[automatically_derived]
#[allow(unused_qualifications)]
impl ::std::clone::Clone for PacketType {
    #[inline]
    fn clone(&self) -> PacketType {
        {
            *self
        }
    }
}
impl ::lain::traits::NewFuzzed for PacketType {
    type RangeType = u8;
    fn new_fuzzed<R: ::lain::rand::Rng>(
        mutator: &mut ::lain::mutator::Mutator<R>,
        mut constraints: Option<&::lain::types::Constraints<Self::RangeType>>,
    ) -> PacketType {
        static weights: [u64; 3usize] = [1u64, 1u64, 1u64];
        #[allow(missing_copy_implementations)]
        #[allow(non_camel_case_types)]
        #[allow(dead_code)]
        struct dist {
            __private_field: (),
        }
        #[doc(hidden)]
        static dist: dist = dist {
            __private_field: (),
        };
        impl ::lazy_static::__Deref for dist {
            type Target = ::lain::rand::distributions::WeightedIndex<u64>;
            fn deref(&self) -> &::lain::rand::distributions::WeightedIndex<u64> {
                #[inline(always)]
                fn __static_ref_initialize() -> ::lain::rand::distributions::WeightedIndex<u64> {
                    ::lain::rand::distributions::WeightedIndex::new(weights.iter()).unwrap()
                }
                #[inline(always)]
                fn __stability() -> &'static ::lain::rand::distributions::WeightedIndex<u64> {
                    static LAZY: ::lazy_static::lazy::Lazy<
                        ::lain::rand::distributions::WeightedIndex<u64>,
                    > = ::lazy_static::lazy::Lazy::INIT;
                    LAZY.get(__static_ref_initialize)
                }
                __stability()
            }
        }
        impl ::lazy_static::LazyStatic for dist {
            fn initialize(lazy: &Self) {
                let _ = &**lazy;
            }
        }
        use ::lain::rand::seq::SliceRandom;
        static options: [PacketType; 3usize] =
            [PacketType::Read, PacketType::Write, PacketType::Reset];
        *options.choose(&mut mutator.rng).unwrap()
    }
}
impl ::lain::traits::Mutatable for PacketType {
    #[allow(unused)]
    fn mutate<R: ::lain::rand::Rng>(
        &mut self,
        mutator: &mut ::lain::mutator::Mutator<R>,
        constraints: Option<&Constraints<u8>>,
    ) {
        *self = <PacketType>::new_fuzzed(mutator, None);
        if mutator.should_fixup() {
            self.fixup(mutator);
        }
    }
}
impl ::lain::traits::PostFuzzerIteration for PacketType {
    fn on_success_for_fields(&self) {}
}
impl ::lain::traits::FixupChildren for PacketType {
    fn fixup_children<R: ::lain::rand::Rng>(&mut self, mutator: &mut Mutator<R>) {}
}
impl ::lain::traits::VariableSizeObject for PacketType {
    fn is_variable_size() -> bool {
        false
    }
}
impl ::lain::traits::ToPrimitive<u32> for PacketType {
    fn to_primitive(&self) -> u32 {
        *self as u32
    }
}
impl ::lain::traits::BinarySerialize for PacketType {
    fn binary_serialize<W: std::io::Write, E: ::lain::byteorder::ByteOrder>(&self, buffer: &mut W) {
        use ::lain::byteorder::{BigEndian, LittleEndian, WriteBytesExt};
        use ::lain::traits::SerializedSize;
        self.to_primitive().binary_serialize::<_, E>(buffer);
    }
}
impl ::lain::traits::SerializedSize for PacketType {
    #[inline(always)]
    fn serialized_size(&self) -> usize {
        use ::lain::traits::SerializedSize;
        {
            let lvl = ::log::Level::Debug;
            if lvl <= ::log::STATIC_MAX_LEVEL && lvl <= ::log::max_level() {
                ::log::__private_api_log(
                    ::std::fmt::Arguments::new_v1(
                        &["getting serialized size of "],
                        &match (&"PacketType",) {
                            (arg0,) => {
                                [::std::fmt::ArgumentV1::new(arg0, ::std::fmt::Display::fmt)]
                            }
                        },
                    ),
                    lvl,
                    &(
                        "example_fuzzer",
                        "example_fuzzer",
                        "example_fuzzer/src/main.rs",
                        50u32,
                    ),
                );
            }
        };
        let size = std::mem::size_of::<PacketType>();
        {
            let lvl = ::log::Level::Debug;
            if lvl <= ::log::STATIC_MAX_LEVEL && lvl <= ::log::max_level() {
                ::log::__private_api_log(
                    ::std::fmt::Arguments::new_v1_formatted(
                        &["size of ", " is 0x"],
                        &match (&"PacketType", &size) {
                            (arg0, arg1) => [
                                ::std::fmt::ArgumentV1::new(arg0, ::std::fmt::Display::fmt),
                                ::std::fmt::ArgumentV1::new(arg1, ::std::fmt::UpperHex::fmt),
                            ],
                        },
                        &[
                            ::std::fmt::rt::v1::Argument {
                                position: ::std::fmt::rt::v1::Position::At(0usize),
                                format: ::std::fmt::rt::v1::FormatSpec {
                                    fill: ' ',
                                    align: ::std::fmt::rt::v1::Alignment::Unknown,
                                    flags: 0u32,
                                    precision: ::std::fmt::rt::v1::Count::Implied,
                                    width: ::std::fmt::rt::v1::Count::Implied,
                                },
                            },
                            ::std::fmt::rt::v1::Argument {
                                position: ::std::fmt::rt::v1::Position::At(1usize),
                                format: ::std::fmt::rt::v1::FormatSpec {
                                    fill: ' ',
                                    align: ::std::fmt::rt::v1::Alignment::Unknown,
                                    flags: 8u32,
                                    precision: ::std::fmt::rt::v1::Count::Implied,
                                    width: ::std::fmt::rt::v1::Count::Is(2usize),
                                },
                            },
                        ],
                    ),
                    lvl,
                    &(
                        "example_fuzzer",
                        "example_fuzzer",
                        "example_fuzzer/src/main.rs",
                        50u32,
                    ),
                );
            }
        };
        return size;
    }
    #[inline(always)]
    fn min_nonzero_elements_size() -> usize {
        std::mem::size_of::<PacketType>()
    }
}
impl Default for PacketType {
    fn default() -> Self {
        PacketType::Read
    }
}
fn main() {
    let mut driver = FuzzerDriver::<GlobalContext>::new(THREAD_COUNT);
    driver.set_global_context(Default::default());
    let driver = Arc::new(driver);
    let ctrlc_driver = driver.clone();
    ctrlc::set_handler(move || {
        ctrlc_driver.signal_exit();
    })
    .expect("couldn't set CTRL-C handler");
    start_fuzzer(driver.clone(), fuzzer_routine);
    driver.join_threads();
    {
        ::std::io::_print(::std::fmt::Arguments::new_v1(
            &["Finished in ", " iterations\n"],
            &match (&driver.num_iterations(),) {
                (arg0,) => [::std::fmt::ArgumentV1::new(arg0, ::std::fmt::Display::fmt)],
            },
        ));
    };
}
fn fuzzer_routine<R: Rng>(
    mutator: &mut Mutator<R>,
    thread_context: &mut FuzzerThreadContext,
    _global_context: Option<Arc<RwLock<GlobalContext>>>,
) -> Result<(), ()> {
    let mut stream =
        TcpStream::connect("127.0.0.1:8080").expect("server isn't running. possible crash?");
    let packet = match thread_context.last_packet {
        Some(ref mut last_packet) => {
            if mutator.mode() == MutatorMode::Havoc {
                last_packet.mutate(mutator, None);
                last_packet
            } else {
                thread_context.scratch_packet = last_packet.clone();
                thread_context.scratch_packet.mutate(mutator, None);
                &thread_context.scratch_packet
            }
        }
        _ => {
            mutator.begin_new_corpus();
            thread_context.last_packet = Some(PacketData::new_fuzzed(mutator, None));
            thread_context.last_packet.as_mut().unwrap()
        }
    };
    let mut serialized_data = Vec::with_capacity(packet.serialized_size());
    packet.binary_serialize::<_, LittleEndian>(&mut serialized_data);
    {
        ::std::io::_print(::std::fmt::Arguments::new_v1(
            &["Sending packet: ", "\n"],
            &match (&packet,) {
                (arg0,) => [::std::fmt::ArgumentV1::new(arg0, ::std::fmt::Debug::fmt)],
            },
        ));
    };
    stream
        .write(&serialized_data)
        .expect("failed to write data");
    let mut response_data = Vec::new();
    stream.read(&mut response_data);
    thread_context.thread_packet_iterations += 1;
    Ok(())
}
