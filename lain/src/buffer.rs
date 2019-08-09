use crate::traits::*;
use crate::types::UnsafeEnum;
use byteorder::{ByteOrder, WriteBytesExt};
use std::io::Write;

/// Default implementation of SerializedSize for slices of items. This runs in O(n) complexity since
/// not all items in the slice are guaranteed to be the same size (e.g. strings)
impl<T> SerializedSize for [T]
where
    T: SerializedSize,
{
    #[inline]
    default fn serialized_size(&self) -> usize {
        trace!("using default serialized_size for array");
        if self.is_empty() {
            return 0;
        }

        let size = self
            .iter()
            .map(SerializedSize::serialized_size)
            .fold(0, |sum, i| sum + i);

        size
    }

    #[inline]
    fn min_nonzero_elements_size() -> usize {
        T::min_nonzero_elements_size()
    }

    #[inline]
    fn max_default_object_size() -> usize {
        T::max_default_object_size()
    }
}

macro_rules! impl_serialized_size_array {
    ( $($size:expr),* ) => {
        $(
            impl<T> SerializedSize for [T; $size]
            where T: SerializedSize {
                #[inline]
                fn serialized_size(&self) -> usize {
                    trace!("using default serialized_size for array");
                    if $size == 0 {
                        return 0;
                    }

                    let size = self
                        .iter()
                        .map(SerializedSize::serialized_size)
                        .fold(0, |sum, i| sum + i);

                    size
                }

                #[inline]
                fn min_nonzero_elements_size() -> usize {
                    T::min_nonzero_elements_size() * $size
                }

                #[inline]
                fn max_default_object_size() -> usize {
                    T::max_default_object_size() * $size
                }
            }
        )*
    }
}

impl_serialized_size_array!(
    0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20, 21, 22, 23, 24, 25, 26,
    27, 28, 29, 30, 31, 32, 33, 34, 35, 36, 37, 38, 39, 40, 41, 42, 43, 44, 45, 46, 47, 48, 49, 50,
    51, 52, 53, 54, 55, 56, 57, 58, 59, 60
);

impl<T> SerializedSize for Vec<T>
where
    T: SerializedSize,
{
    #[inline]
    fn serialized_size(&self) -> usize {
        trace!("getting serialized size for Vec");
        if self.is_empty() {
            trace!("returning 0 since there's no elements");
            return 0;
        }

        let size = self.iter().map(SerializedSize::serialized_size).sum();

        trace!("size is 0x{:02X}", size);

        size
    }

    #[inline]
    fn min_nonzero_elements_size() -> usize {
        T::min_nonzero_elements_size()
    }

    #[inline]
    fn max_default_object_size() -> usize {
        T::max_default_object_size()
    }
}

impl SerializedSize for str {
    #[inline]
    fn serialized_size(&self) -> usize {
        trace!("getting serialized size of str");
        self.len()
    }

    #[inline]
    fn min_nonzero_elements_size() -> usize {
        1
    }

    #[inline]
    fn max_default_object_size() -> usize {
        1
    }
}

impl SerializedSize for String {
    #[inline]
    fn serialized_size(&self) -> usize {
        trace!("getting serialized size of String");
        self.len()
    }

    #[inline]
    fn min_nonzero_elements_size() -> usize {
        1
    }

    #[inline]
    fn max_default_object_size() -> usize {
        1
    }
}

impl<T> BinarySerialize for Vec<T>
where
    T: BinarySerialize,
{
    fn binary_serialize<W: Write, E: ByteOrder>(&self, buffer: &mut W) -> usize {
        let inner_ref: &[T] = self.as_ref();
        inner_ref.binary_serialize::<_, E>(buffer)
    }
}

impl BinarySerialize for bool {
    #[inline(always)]
    default fn binary_serialize<W: Write, E: ByteOrder>(&self, buffer: &mut W) -> usize {
        // unsafe code here for non-binary booleans. i.e. when we do unsafe mutations
        // sometimes a bool is represented as 3 or some other non-0/1 number
        let value = unsafe { *((self as *const bool) as *const u8) };

        buffer.write_u8(value).unwrap();
        std::mem::size_of::<u8>()
    }
}

impl BinarySerialize for i8 {
    #[inline(always)]
    fn binary_serialize<W: Write, E: ByteOrder>(&self, buffer: &mut W) -> usize {
        buffer.write_i8(*self as i8).unwrap();
        std::mem::size_of::<i8>()
    }
}

impl BinarySerialize for u8 {
    #[inline(always)]
    fn binary_serialize<W: Write, E: ByteOrder>(&self, buffer: &mut W) -> usize {
        buffer.write_u8(*self as u8).unwrap();
        std::mem::size_of::<u8>()
    }
}

impl BinarySerialize for [u8] {
    #[inline(always)]
    fn binary_serialize<W: Write, E: ByteOrder>(&self, buffer: &mut W) -> usize{
        buffer.write(&self).unwrap()
    }
}

impl<T> BinarySerialize for [T]
where
    T: BinarySerialize,
{
    #[inline(always)]
    default fn binary_serialize<W: Write, E: ByteOrder>(&self, buffer: &mut W) -> usize {
        let mut bytes_written = 0;
        for item in self.iter() {
            bytes_written += item.binary_serialize::<W, E>(buffer);
        }

        bytes_written
    }
}

impl<T, I> BinarySerialize for UnsafeEnum<T, I>
where
    T: BinarySerialize,
    I: BinarySerialize + Clone,
{
    default fn binary_serialize<W: Write, E: ByteOrder>(&self, buffer: &mut W) -> usize {
        match *self {
            UnsafeEnum::Invalid(ref value) => {
                value.binary_serialize::<_, E>(buffer)
            }
            UnsafeEnum::Valid(ref value) => {
                value.binary_serialize::<_, E>(buffer)
            }
        }
    }
}

impl BinarySerialize for String {
    #[inline(always)]
    fn binary_serialize<W: Write, E: ByteOrder>(&self, buffer: &mut W) -> usize {
        self.as_bytes().binary_serialize::<_, E>(buffer)
    }
}

/// This probably could and should be on a generic impl where T: Deref, but currently
/// this causes a specialization issue since other crates could impl Deref<Target=T> for
/// bool (specifically) in the future. See: https://github.com/rust-lang/rust/issues/45542
impl BinarySerialize for &str {
    #[inline(always)]
    fn binary_serialize<W: Write, E: ByteOrder>(&self, buffer: &mut W) -> usize {
        self.as_bytes().binary_serialize::<_, E>(buffer)
    }
}

macro_rules! impl_binary_serialize {
    ( $($name:ident),* ) => {
        $(
            impl BinarySerialize for $name {
                #[inline(always)]
                fn binary_serialize<W: Write, E: ByteOrder>(&self, buffer: &mut W) -> usize {
                    // need to use mashup here to do write_(u8|u16|...) since you can't concat
                    // idents otherwise
                    mashup! {
                        m["method_name"] = write_ $name;
                    }

                    m! {
                        buffer."method_name"::<E>(*self as $name).unwrap();
                    }
                    std::mem::size_of::<$name>()
                }
            }
        )*
    }
}

impl_binary_serialize!(i64, u64, i32, u32, i16, u16, f32, f64);

macro_rules! impl_serialized_size {
    ( $($name:ident),* ) => {
        $(
            impl SerializedSize for $name {
                #[inline(always)]
                fn serialized_size(&self) -> usize {
                    std::mem::size_of::<$name>()
                }

                #[inline]
                fn min_nonzero_elements_size() -> usize {
                    std::mem::size_of::<$name>()
                }

                #[inline]
                fn max_default_object_size() -> usize {
                    std::mem::size_of::<$name>()
                }
            }
        )*
    }
}

impl_serialized_size!(i64, u64, i32, u32, i16, u16, f32, f64, u8, i8, bool);

impl<T, U> SerializedSize for T
where T: ToPrimitive<Output=U>
{
    #[inline]
    default fn serialized_size(&self) -> usize {
        std::mem::size_of::<U>()
    }

    #[inline]
    default fn min_nonzero_elements_size() -> usize {
        std::mem::size_of::<U>()
    }

    #[inline]
    default fn max_default_object_size() -> usize {
        std::mem::size_of::<U>()
    }
}

impl SerializedSize for &str {
    #[inline]
    fn serialized_size(&self) -> usize {
        self.len()
    }

    #[inline]
    fn min_nonzero_elements_size() -> usize {
        1
    }

    #[inline]
    fn max_default_object_size() -> usize {
        1
    }
}