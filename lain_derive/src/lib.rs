// required for using the quote!{} macro for large invocations
#![recursion_limit = "256"]

extern crate proc_macro;
extern crate proc_macro2;
extern crate syn;
#[macro_use]
extern crate log;

use quote::quote;

use proc_macro2::TokenStream;
use syn::{parse_macro_input, DeriveInput};

//mod fuzzerobject;
mod mutations;
mod serialize;
mod internals;
mod dummy;

//use crate::fuzzerobject::*;
//use crate::serialize::binary_serialize_helper;
use quote::quote_spanned;
use syn::spanned::Spanned;
use syn::{Data, Fields};

fn to_compile_errors(errors: Vec<syn::Error>) -> proc_macro2::TokenStream {
    let compile_errors = errors.iter().map(syn::Error::to_compile_error);
    quote!(#(#compile_errors)*)
}

/// Implements [rand::distributions::Standard] for enums that derive this trait.
/// This will allow you to use `rand::gen()` to randomly select an enum value.
/// # Example
///
/// ```compile_fail
/// extern crate rand;
///
/// #[derive(NewFuzzed)]
/// enum Foo {
///     Bar,
///     Baz,
/// }
///
/// let choice: Foo = rand::gen();
/// ```
#[proc_macro_derive(NewFuzzed, attributes(lain))]
pub fn new_fuzzed(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    mutations::expand_new_fuzzed(&input).unwrap_or_else(to_compile_errors).into()
}

/// Implements [lain::traits::BinarySerialize] on the given struct/enum.
/// The byteorder of fields can be overridden with `#[byteorder(big)]` or
/// `#[byteorder(little)]`
///
/// # Example
///
/// ```compile_fail
/// extern crate lain;
///
/// use lain::{BinarySerialize, hexdump};
///
/// use std::io::BufWriter;
///
///#[derive(BinarySerialize)]
///struct LittleEndianStruct {
///    little_endian_field: u32,
///}
///
///#[derive(BinarySerialize)]
///struct MyStruct {
///    field1: u32,
///    #[byteorder(little)]
///    field2: LittleEndianStruct,
///}
///
///fn serialize_struct() {
///    let s = MyStruct {
///        field1: 0xAABBCCDD,
///        field2: AlwaysLittleEndianStruct {
///            little_endian_field: 0x00112233,
///        },
///    };
///
///    let mut serialized_buffer = [0u8; std::mem::size_of::<MyStruct>()];
///    {
///        let buffer_ref: &mut [u8] = &mut serialized_buffer;
///        let mut writer = BufWriter::new(buffer_ref);
///        s.binary_serialize::<_, lain::byteorder::BigEndian>(&mut writer);
///    }
///    println!("{}", &hexdump(serialized_buffer.iter()));
///    // Output:
///    // ------00 01 02 03 04 05 06 07 08 09 0A 0B 0C 0D 0E 0F
///    // 0000: AA BB CC DD 33 22 11 00
///}
/// ```
#[proc_macro_derive(
    BinarySerialize,
    attributes(lain)
)]
pub fn binary_serialize(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    serialize::expand_binary_serialize(&input).unwrap_or_else(to_compile_errors).into()
}

/// Automatically implements [trait@lain::traits::Mutatable] with basic
/// randomization
///
/// # Notes
///
/// - Any bitfields will automatically be set within the appropriate ranges.
/// - Min/max values for primitives can be specified using `#[lain(min = 10, max = 20)]`.
/// - Fields can be ignored using #[lain(ignore)].
/// - Custom initializers can be specified using #[lain(initializer = "my_initializer_func()")]
///
/// # Example
///
/// ```compile_fail
/// extern crate lain;
/// use lain::prelude::*;
/// use lain::rand;
///
/// #[derive(RandomChoice)]
/// enum ChoiceValue {
///     FirstChoice = 1,
///     SecondChoice = 2,
/// }
///
/// #[derive(Default, Mutatable)]
/// struct Foo {
///     field1: u8,
///     #[lain(bits = 7)]
///     field2: u8,
///     #[lain(bits = 1)]
///     field3: u8,
///     #[lain(max = 300)]
///     field4: u32,
///     choice_field: ChoiceValue,
/// }
///
/// let mutator = Mutator::new(rand::thread_rng());
/// let my_struct: Foo = Default::default();
/// my_struct.mutate()
/// ```
#[proc_macro_derive(Mutatable, attributes(lain))]
pub fn mutatable_helper(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    mutations::expand_mutatable(&input).unwrap_or_else(to_compile_errors).into()
}

/// Automatically implements [trait@lain::traits::VariableSizeObject]
///
/// # Example
///
/// ```compile_fail
/// extern crate lain;
/// use lain::prelude::*;
/// use lain::rand;
///
/// #[derive(Default, NewFuzzed, VariableSizeObject)]
/// struct Foo {
///     field1: u8,
///     field2: Vec<u8>,
/// }
///
/// let mutator = Mutator::new(rand::thread_rng());
/// let my_struct: Foo = Default::default();
/// my_struct.mutate()
/// ```
#[proc_macro_derive(VariableSizeObject)]
pub fn variable_size_object(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let input = parse_macro_input!(input as DeriveInput);

    let expanded = variable_size_object_helper(&input);

    // Uncomment to dump the AST
    // println!("{}", expanded);

    proc_macro::TokenStream::from(expanded)
}

fn variable_size_object_helper(input: &DeriveInput) -> TokenStream {
    // TODO: refactor this to use the same pattern as Mutatable/NewFuzzed introduced
    // in v0.2.0
    let imp: TokenStream;

    match input.data {
        Data::Enum(ref data) => {
            let mut simple_variants = true;
            for variant in &data.variants {
                match variant.fields {
                    Fields::Unit => {
                        continue;
                    }
                    _ => {
                        simple_variants = false;
                    }
                }
            }

            imp = if simple_variants {
                quote! {false}
            } else {
                quote! {true}
            };
        }
        Data::Struct(ref data) => {
            if let Fields::Named(ref fields) = data.fields {
                if fields.named.len() == 0 {
                    imp = quote! {false};
                } else {
                    let mut tokens = quote! {false};

                    for field in fields.named.iter() {
                        let ty = &field.ty;
                        tokens.extend(quote_spanned! { field.span() =>
                            || <#ty>::is_variable_size()
                        });
                    }

                    imp = tokens;
                }
            } else {
                panic!("Need to add support for unnamed struct fields");
            }
        }
        _ => panic!("Non-enum/struct data types are not supported"),
    }

    let name = &input.ident;

    let (impl_generics, ty_generics, where_clause) = input.generics.split_for_impl();

    quote! {
        impl #impl_generics ::lain::traits::VariableSizeObject for #name #ty_generics #where_clause {
            fn is_variable_size() -> bool {
                #imp
            }
        }
    }
}

/// A "catch-all" derive for NewFuzzed, Mutatable, and VariableObjectSize
#[proc_macro_derive(FuzzerObject, attributes(lain))]
pub fn fuzzer_object(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let mut base_token_stream = TokenStream::new();

    let input = parse_macro_input!(input as DeriveInput);
    base_token_stream.extend::<TokenStream>(mutations::expand_new_fuzzed(&input).unwrap_or_else(to_compile_errors).into());
    base_token_stream.extend::<TokenStream>(mutations::expand_mutatable(&input).unwrap_or_else(to_compile_errors).into());
    base_token_stream.extend::<TokenStream>(variable_size_object_helper(&input));

    base_token_stream.into()
}

/// Implements `ToPrimitive<u8>` for the given enum.
#[proc_macro_derive(ToPrimitiveU8)]
pub fn to_primitive_u8(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    to_primitive_of_type(input, quote! {u8})
}

/// Implements `ToPrimitive<u16>` for the given enum.
#[proc_macro_derive(ToPrimitiveU16)]
pub fn to_primitive_u16(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    to_primitive_of_type(input, quote! {u16})
}

/// Implements `ToPrimitive<u32>` for the given enum.
#[proc_macro_derive(ToPrimitiveU32)]
pub fn to_primitive_u32(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    to_primitive_of_type(input, quote! {u32})
}

/// Implements `ToPrimitive<u64>` for the given enum.
#[proc_macro_derive(ToPrimitiveU64)]
pub fn to_primitive_u64(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    to_primitive_of_type(input, quote! {u64})
}

fn to_primitive_of_type(
    input: proc_macro::TokenStream,
    ty: proc_macro2::TokenStream,
) -> proc_macro::TokenStream {
    let input = parse_macro_input!(input as DeriveInput);

    let name = input.ident;

    let (impl_generics, ty_generics, where_clause) = input.generics.split_for_impl();

    let expanded = quote! {
        impl #impl_generics ::lain::traits::ToPrimitive for #name #ty_generics #where_clause {
            type Output = #ty;

            fn to_primitive(&self) -> #ty {
                *self as #ty
            }
        }
    };

    // Uncomment to dump the AST
    debug!("{}", expanded);

    proc_macro::TokenStream::from(expanded)
}
