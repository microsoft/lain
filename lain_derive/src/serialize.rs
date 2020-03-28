use proc_macro2::{TokenStream};
use std::str::FromStr;
use syn::spanned::Spanned;
use syn::export::quote::ToTokens;
use quote::{quote, quote_spanned};

use crate::internals::{Ctxt, Derive};
use crate::internals::ast::{Container, Data, Field, Variant, Style, is_primitive_type};
use crate::dummy;

struct SerializedSizeBodies {
    serialized_size: TokenStream,
    min_nonzero_elements_size: TokenStream,
    max_default_object_size: TokenStream,
    min_enum_variant_size: TokenStream,
}

#[derive(Copy, Clone, PartialEq)]
enum SerializedSizeVisitorType {
    SerializedSize,
    MinNonzeroElements,
    MaxDefaultObjectSize,
    MinEnumVariantSize,
}

pub fn expand_binary_serialize(input: &syn::DeriveInput) -> Result<TokenStream, Vec<syn::Error>> {
    let ctx = Ctxt::new();

    let cont = match Container::from_ast(&ctx, input, Derive::NewFuzzed) {
        Some(cont) => cont,
        None => return Err(ctx.check().unwrap_err()),
    };

    ctx.check()?;

    let ident = &cont.ident;
    let ident_as_string = ident.to_string();
    let (impl_generics, ty_generics, where_clause) = input.generics.split_for_impl();

    let serialize_body = binary_serialize_body(&cont);
    let SerializedSizeBodies { serialized_size, min_nonzero_elements_size, max_default_object_size, min_enum_variant_size } = serialized_size_body(&cont, cont.attrs.serialized_size(), cont.attrs.min_serialized_size());

    let lain = cont.attrs.lain_path();

    let impl_block = quote! {
        #[allow(clippy)]
        #[allow(unknown_lints)]
        #[automatically_derived]
        impl #impl_generics #lain::traits::BinarySerialize for #ident #ty_generics #where_clause {
            fn binary_serialize<W: std::io::Write, E: #lain::byteorder::ByteOrder>(&self, buffer: &mut W) -> usize {
                use #lain::traits::SerializedSize;
                use #lain::byteorder::{LittleEndian, BigEndian, WriteBytesExt};

                let mut bytes_written = 0;

                #serialize_body

                if bytes_written < self.serialized_size() {
                    let padding_bytes = std::cmp::max(self.serialized_size(), Self::min_nonzero_elements_size()) - bytes_written;
                    if padding_bytes != 0 {
                        let null = 0x0u8;
                        for _i in 0..padding_bytes {
                            bytes_written += null.binary_serialize::<_, E>(buffer);
                        }
                    }
                }

                bytes_written
            }
        }

        // TODO: Split this into its own derive
        #[allow(clippy)]
        #[allow(unknown_lints)]
        #[automatically_derived]
        impl #impl_generics #lain::traits::SerializedSize for #ident #ty_generics #where_clause {
            #[inline]
            fn serialized_size(&self) -> usize {
                use #lain::traits::SerializedSize;
                #lain::log::debug!("getting serialized size of {}", #ident_as_string);
                let size = #serialized_size;

                let size = if size < Self::min_nonzero_elements_size() {
                    Self::min_nonzero_elements_size()
                } else {
                    size
                };
                #lain::log::debug!("size of {} is 0x{:02X}", #ident_as_string, size);

                return size;
            }

            #[inline]
            fn min_nonzero_elements_size() -> usize {
                #min_nonzero_elements_size
            }

            #[inline]
            fn max_default_object_size() -> usize {
                #max_default_object_size
            }

            #[inline]
            fn min_enum_variant_size(&self) -> usize {
                #min_enum_variant_size
            }
        }
    };

    let data = dummy::wrap_in_const("BINARYSERIALIZE", ident, impl_block);

    Ok(data)
}

fn binary_serialize_body(cont: &Container) -> TokenStream {
    match cont.data {
        Data::Enum(ref variants) if variants[0].style != Style::Unit => binary_serialize_enum(variants, &cont.ident),
        Data::Enum(ref _variants) => binary_serialize_unit_enum(&cont.ident),
        Data::Struct(Style::Struct, ref fields) | Data::Struct(Style::Tuple, ref fields) => binary_serialize_struct(fields),
        Data::Struct(Style::Unit, ref _fields) => TokenStream::new(),
    }
}

fn serialized_size_body(cont: &Container, size: Option<usize>, min_size: Option<usize>) -> SerializedSizeBodies {
    if let Some(size) = size.clone() {
        let size_tokens = quote!{#size};

        return SerializedSizeBodies {
            serialized_size: size_tokens.clone(),
            min_nonzero_elements_size: size_tokens.clone(),
            max_default_object_size: size_tokens.clone(),
            min_enum_variant_size: size_tokens.clone(),
        };
    }

    match cont.data {
        Data::Enum(ref variants) if variants[0].style != Style::Unit => serialized_size_enum(variants, &cont.ident, size, min_size),
        Data::Enum(ref _variants) => serialized_size_unit_enum(&cont.ident),
        Data::Struct(Style::Struct, ref fields) | Data::Struct(Style::Tuple, ref fields) => serialized_size_struct(fields),
        Data::Struct(Style::Unit, ref _fields) => {
            let zero_size = quote!{0};
            SerializedSizeBodies {
                serialized_size: zero_size.clone(),
                min_nonzero_elements_size: zero_size.clone(),
                max_default_object_size: zero_size.clone(),
                min_enum_variant_size: zero_size,
            }
        }
    }
}

fn binary_serialize_enum (
    variants: &[Variant],
    cont_ident: &syn::Ident,
) ->  TokenStream {
    let match_arms = binary_serialize_enum_visitor(variants, cont_ident);

    quote! {
        let mut bitfield: u64 = 0;

        match *self {
            #(#match_arms)*
        }
    }
}

fn binary_serialize_unit_enum(cont_ident: &syn::Ident) -> TokenStream {
    quote! {
        bytes_written += <<#cont_ident as _lain::traits::ToPrimitive>::Output>::binary_serialize::<_, E>(&self.to_primitive(), buffer);
    }
}

fn binary_serialize_struct (
    fields: &[Field],
) -> TokenStream {
    let serializers = binary_serialize_struct_visitor(fields);

    quote! {
        let mut bitfield: u64 = 0;

        #(#serializers)*
    }
}

fn binary_serialize_struct_visitor(
    fields: &[Field],
) -> Vec<TokenStream> {
    fields
        .iter()
        .map(|field| {
            let (_field_ident, _field_ident_string, serializer) = field_serializer(field, "self.", false);

            serializer
        })
        .collect()
}

fn field_serializer(field: &Field, name_prefix: &'static str, is_destructured: bool) -> (TokenStream, String, TokenStream) {
    let ty = &field.ty;
    let field_ident_string = match field.member{
        syn::Member::Named(ref ident) => ident.to_string(),
        syn::Member::Unnamed(ref idx) => idx.index.to_string(),
    };

    let value_ident = TokenStream::from_str(&format!("{}{}", name_prefix, field_ident_string)).unwrap();
    let borrow = if is_destructured {
        TokenStream::new()
    } else {
        quote!{&}
    };

    let endian = if field.attrs.big_endian() {
        quote!{_lain::byteorder::BigEndian}
    } else if field.attrs.little_endian() {
        quote!{_lain::byteorder::LittleEndian}
    } else {
        // inherit
        quote!{E}
    };

    let serialize_stmts = if let Some(bits) = field.attrs.bits() {
        let bit_mask = 2_u64.pow(bits as u32) - 1;
        let bit_shift = field.attrs.bit_shift().unwrap();
        let is_last_field = field.attrs.is_last_field();

        let bitfield_type = field.attrs.bitfield_type().unwrap_or(&field.ty);

        let type_total_bits = if is_primitive_type(bitfield_type, "u8") {
            8
        } else if is_primitive_type(&bitfield_type, "u16") {
            16
        } else if is_primitive_type(&bitfield_type, "u32") {
            32
        } else if is_primitive_type(&bitfield_type, "u64") {
            64
        } else {
            panic!("got to field_serialize with an unsupported bitfield type `{}`. ensure that checks in ast code are correct", bitfield_type.into_token_stream());
        };

        let bitfield_value = if field.attrs.bitfield_type().is_some() {
            quote_spanned! {field.ty.span() => #value_ident.to_primitive()}
        } else {
            quote_spanned!{field.original.span() => #value_ident}
        };

        let mut bitfield_setter = quote_spanned!{ field.ty.span() =>
            bitfield |= ((#bitfield_value as #bitfield_type & #bit_mask as #bitfield_type) << #bit_shift) as u64;
        };

        if bits + bit_shift == type_total_bits || is_last_field {
            bitfield_setter.extend(quote_spanned!{field.ty.span() => bytes_written += <#bitfield_type>::binary_serialize::<_, #endian>(&(bitfield as #bitfield_type), buffer);});
        }

        bitfield_setter
    } else {
        if let syn::Type::Array(ref _a) = ty {
            // TODO: Change this once const generics are stabilized
            quote_spanned! { field.original.span() =>
                bytes_written += #value_ident.binary_serialize::<_, #endian>(buffer);
            }
        } else {
            quote_spanned! { field.original.span() =>
                bytes_written += <#ty>::binary_serialize::<_, #endian>(#borrow#value_ident, buffer);
            }
        }
    };

    (value_ident, field_ident_string, serialize_stmts)
}

fn binary_serialize_enum_visitor(
    variants: &[Variant],
    cont_ident: &syn::Ident,
) -> Vec<TokenStream> {
    let match_arms = variants
        .iter()
        .map(|variant| {
            let variant_ident = &variant.ident;
            let full_ident = quote!{#cont_ident::#variant_ident};
            let mut field_identifiers = vec![];

            let field_serializers: Vec<TokenStream> = variant.fields.iter().map(|field| {
                let (value_ident, _field_ident_string, initializer) = field_serializer(field, "__field", true);
                field_identifiers.push(quote_spanned!{ field.member.span() => #value_ident });

                initializer
            })
            .collect();

            let match_arm = quote! {
                #full_ident(#(ref #field_identifiers,)*) => {
                    #(#field_serializers)*
                }
            };

            match_arm
        })
        .collect();

    match_arms
}

fn serialized_size_enum (
    variants: &[Variant],
    cont_ident: &syn::Ident,
    size: Option<usize>,
    min_size: Option<usize>,
) ->  SerializedSizeBodies {
    let match_arms = serialized_size_enum_visitor(variants, cont_ident, SerializedSizeVisitorType::SerializedSize);
    let nonzero_variants = serialized_size_enum_visitor(variants, cont_ident, SerializedSizeVisitorType::MinNonzeroElements);
    let max_obj = serialized_size_enum_visitor(variants, cont_ident, SerializedSizeVisitorType::MaxDefaultObjectSize);
    let min_variant = serialized_size_enum_visitor(variants, cont_ident, SerializedSizeVisitorType::MinEnumVariantSize);

    let serialized_size = if let Some(size) = size {
        quote!{#size}
    } else {
        quote! {
            match *self {
                #(#match_arms)*
            }
        }
    };
    
    let min_nonzero = if let Some(min_size) = min_size {
        quote! {#min_size}
    } else {
        quote! {*[#(#nonzero_variants,)*].iter().min_by(|a, b| a.cmp(b)).unwrap()}
    };

    let max_default = quote! {*[#(#max_obj,)*].iter().max_by(|a, b| a.cmp(b)).unwrap()};

    let min_variant = quote! {
        match *self {
            #(#min_variant)*
        }
    };

    SerializedSizeBodies {
        serialized_size,
        min_nonzero_elements_size: min_nonzero,
        max_default_object_size: max_default,
        min_enum_variant_size: min_variant,
    }
}

fn serialized_size_unit_enum(cont_ident: &syn::Ident) -> SerializedSizeBodies {
    let size = quote! {
        std::mem::size_of::<<#cont_ident as _lain::traits::ToPrimitive>::Output>()
    };

    SerializedSizeBodies {
        serialized_size: size.clone(),
        min_nonzero_elements_size: size.clone(),
        max_default_object_size: size.clone(),
        min_enum_variant_size: size,
    }
}

fn serialized_size_struct (
    fields: &[Field],
) -> SerializedSizeBodies {
    let serialized_size = serialized_size_struct_visitor(fields, SerializedSizeVisitorType::SerializedSize);

    let min_nonzero = serialized_size_struct_visitor(fields, SerializedSizeVisitorType::MinNonzeroElements);

    let max_default = serialized_size_struct_visitor(fields, SerializedSizeVisitorType::MaxDefaultObjectSize);

    SerializedSizeBodies {
        serialized_size: quote! {0 #(+#serialized_size)* },
        min_nonzero_elements_size: quote! { 0 #(+#min_nonzero)* },
        max_default_object_size: quote! {0 #(+#max_default)*},
        min_enum_variant_size: quote! {Self::min_nonzero_elements_size()},
    }
}

fn serialized_size_struct_visitor(
    fields: &[Field],
    visitor_type: SerializedSizeVisitorType,
) -> Vec<TokenStream> {
    fields
        .iter()
        .map(|field| {
            let (_field_ident, _field_ident_string, serialized_size) = field_serialized_size(field, "self.", false, visitor_type);

            serialized_size
        })
        .collect()
}

fn field_serialized_size(field: &Field, name_prefix: &'static str, is_destructured: bool, visitor_type: SerializedSizeVisitorType) -> (TokenStream, String, TokenStream) {
    let ty = &field.ty;
    let field_ident_string = match field.member{
        syn::Member::Named(ref ident) => ident.to_string(),
        syn::Member::Unnamed(ref idx) => idx.index.to_string(),
    };

    let value_ident = TokenStream::from_str(&format!("{}{}", name_prefix, field_ident_string)).unwrap();
    let borrow = if is_destructured {
        TokenStream::new()
    } else {
        quote!{&}
    };

    let serialized_size_stmts = if let Some(bits) = field.attrs.bits() {
        let bit_shift = field.attrs.bit_shift().unwrap();
        let bitfield_type = field.attrs.bitfield_type().unwrap_or(&field.ty);
        let is_last_field= field.attrs.is_last_field();

        let type_total_bits = if is_primitive_type(bitfield_type, "u8") {
            8
        } else if is_primitive_type(&bitfield_type, "u16") {
            16
        } else if is_primitive_type(&bitfield_type, "u32") {
            32
        } else if is_primitive_type(&bitfield_type, "u64") {
            64
        } else {
            panic!("got to field_serialize_size with an unsupported bitfield type `{}`. ensure that checks in ast code are correct", bitfield_type.into_token_stream());
        };

        let bitfield_value = if field.attrs.bitfield_type().is_some() {
            quote_spanned! {field.original.span() => #borrow#value_ident.to_primitive()}
        } else {
            quote_spanned!{ field.original.span() => #borrow#value_ident}
        };

        // kind of a hack but only emit the size of the bitfield for the first element
        if bits + bit_shift == type_total_bits || is_last_field {
            match visitor_type {
                SerializedSizeVisitorType::SerializedSize => quote_spanned!{field.original.span() => _lain::traits::SerializedSize::serialized_size(#bitfield_value)},
                SerializedSizeVisitorType::MinNonzeroElements | SerializedSizeVisitorType::MinEnumVariantSize => quote_spanned!{field.original.span() => <#bitfield_type>::min_nonzero_elements_size()},
                SerializedSizeVisitorType::MaxDefaultObjectSize => quote_spanned!{field.original.span() => <#bitfield_type>::max_default_object_size()},
            }
        } else {
            quote!{0 /* bitfield */}
        }
    } else {
        match visitor_type {
            SerializedSizeVisitorType::SerializedSize => quote_spanned!{ field.original.span() => _lain::traits::SerializedSize::serialized_size(#borrow#value_ident)},
            SerializedSizeVisitorType::MinNonzeroElements | SerializedSizeVisitorType::MinEnumVariantSize  => {
                match ty {
                    syn::Type::Path(ref p) if p.path.segments[0].ident == "Vec" && field.attrs.min().is_some() => {
                        let min = field.attrs.min().unwrap();
                        quote_spanned!{ field.original.span() => <#ty>::min_nonzero_elements_size() * #min }
                    },
                    _ => {
                            quote_spanned!{ field.original.span() => (<#ty>::min_nonzero_elements_size() ) }
                    }
                }
            }
            SerializedSizeVisitorType::MaxDefaultObjectSize => {
                match ty {
                    syn::Type::Path(ref p) if p.path.segments[0].ident == "Vec" && field.attrs.max().is_some() => {
                        let max = field.attrs.max().unwrap();
                        quote_spanned!{ field.original.span() => <#ty>::max_default_object_size() * #max }
                    },
                    _ => {
                            quote_spanned!{ field.original.span() => (<#ty>::max_default_object_size() ) }
                    }
                }
            },
        }
    };

    (value_ident, field_ident_string, serialized_size_stmts)
}

fn serialized_size_enum_visitor(
    variants: &[Variant],
    cont_ident: &syn::Ident,
    visitor_type: SerializedSizeVisitorType
) -> Vec<TokenStream> {
    let match_arms = variants
        .iter()
        .map(|variant| {
            let variant_ident = &variant.ident;
            let full_ident = quote!{#cont_ident::#variant_ident};
            let mut field_identifiers = vec![];

            let field_sizes: Vec<TokenStream> = variant.fields.iter().map(|field| {
                let (value_ident, _field_ident_string, field_size) = field_serialized_size(field, "__field", true, visitor_type);
                field_identifiers.push(quote_spanned!{ field.member.span() => #value_ident });

                field_size
            })
            .collect();

            match visitor_type {
                SerializedSizeVisitorType::SerializedSize | SerializedSizeVisitorType::MinEnumVariantSize  => {
                    quote_spanned! { variant.original.span() =>
                        #full_ident(#(ref #field_identifiers,)*) => {
                            0 #(+#field_sizes)*
                        }
                    }
                }
                _ => quote_spanned! { variant.original.span() =>
                    0 #(+#field_sizes)*
                }
            }
        })
        .collect();

    match_arms
}
