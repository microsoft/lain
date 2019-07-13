use crate::rand::Rng;
use crate::traits::*;

static DANGEROUS_NUMBERS_U8: &'static [u8] = &[
    std::u8::MIN,             // 0x00
    std::u8::MAX,             // 0xff
    std::i8::MAX as u8,       // 0x7f
    (std::i8::MAX as u8) + 1, // 0x80
];

static DANGEROUS_NUMBERS_U16: &'static [u16] = &[
    // big-endian variants
    std::u16::MIN,              // 0x0000
    std::u16::MAX,              // 0xffff
    std::i16::MAX as u16,       // 0x7fff
    (std::i16::MAX as u16) + 1, // 0x8000
    // little-endian variants
    0xff7f,
    0x0080,
];

static DANGEROUS_NUMBERS_U32: &'static [u32] = &[
    // big-endian variants
    std::u32::MIN,
    std::u32::MAX,
    std::i32::MAX as u32,
    (std::i32::MAX as u32) + 1,
    // little-endian variants
    0xffff_ff7f,
    0x0000_0080,
];

static DANGEROUS_NUMBERS_U64: &'static [u64] = &[
    // big-endian variants
    std::u64::MIN,
    std::u64::MAX,
    std::i64::MAX as u64,
    (std::i64::MAX as u64) + 1,
    // little-endian variants
    0xffff_ffff_ffff_ff7f,
    0x0000_0000_0000_0080,
];

static DANGEROUS_NUMBERS_F32: &'static [f32] = &[
    std::f32::INFINITY,
    std::f32::MAX,
    std::f32::MIN,
    std::f32::MIN_POSITIVE,
    std::f32::NAN,
    std::f32::NEG_INFINITY,
];

static DANGEROUS_NUMBERS_F64: &'static [f64] = &[
    std::f64::INFINITY,
    std::f64::MAX,
    std::f64::MIN,
    std::f64::MIN_POSITIVE,
    std::f64::NAN,
    std::f64::NEG_INFINITY,
];

macro_rules! dangerous_number {
    ( $ty:ident, $nums:ident ) => {
        impl DangerousNumber<$ty> for $ty {
            fn select_dangerous_number<R: Rng>(rng: &mut R) -> $ty {
                return $nums[rng.gen_range(0, $nums.len())] as $ty;
            }

            fn dangerous_number_at_index(idx: usize) -> $ty {
                $nums[idx] as $ty
            }

            fn dangerous_numbers_len() -> usize {
                $nums.len()
            }
        }
    };
}

dangerous_number!(u8, DANGEROUS_NUMBERS_U8);
dangerous_number!(i8, DANGEROUS_NUMBERS_U8);
dangerous_number!(u16, DANGEROUS_NUMBERS_U16);
dangerous_number!(i16, DANGEROUS_NUMBERS_U16);
dangerous_number!(u32, DANGEROUS_NUMBERS_U32);
dangerous_number!(i32, DANGEROUS_NUMBERS_U32);
dangerous_number!(u64, DANGEROUS_NUMBERS_U64);
dangerous_number!(i64, DANGEROUS_NUMBERS_U64);
dangerous_number!(f32, DANGEROUS_NUMBERS_F32);
dangerous_number!(f64, DANGEROUS_NUMBERS_F64);
