use proc_macro2::{Span, TokenStream};
use std::str::FromStr;
use syn::spanned::Spanned;
use syn::{parse_macro_input, DeriveInput, Lit, NestedMeta};
use quote::{quote, quote_spanned, ToTokens};

use crate::internals::{Ctxt, Derive, attr};
use crate::internals::ast::{Container, Data, Field, Variant, Style, is_primitive_type};
use crate::dummy;

#[derive(Copy, Clone, PartialEq)]
enum SerializedSizeVisitorType {
    SerializedSize,
    MinNonzeroElements,
}

pub fn expand_binary_serialize(input: &syn::DeriveInput) -> Result<TokenStream, Vec<syn::Error>> {
    let ctx = Ctxt::new();

    let cont = match Container::from_ast(&ctx, input, Derive::NewFuzzed) {
        Some(cont) => cont,
        None => return Err(ctx.check().unwrap_err()),
    };

    let ident = &cont.ident;
    let ident_as_string = ident.to_string();
    let (impl_generics, ty_generics, where_clause) = input.generics.split_for_impl();

    let serialize_body = binary_serialize_body(&cont);
    let (serialized_size_body, min_nonzero_elements_body) = if let Some(size) = cont.attrs.serialized_size(){
        let size = quote!{#size};
        (size.clone(), size)
    } else {
        serialized_size_body(&cont)
    };

    let lain = cont.attrs.lain_path();

    ctx.check()?;

    let impl_block = quote! {
        #[automatically_derived]
        impl #impl_generics #lain::traits::BinarySerialize for #ident #ty_generics #where_clause {
            fn binary_serialize<W: std::io::Write, E: #lain::byteorder::ByteOrder>(&self, buffer: &mut W) {
                use #lain::traits::SerializedSize;
                use #lain::byteorder::{LittleEndian, BigEndian, WriteBytesExt};

                #serialize_body
            }
        }

        // TODO: Split this into its own derive
        impl #impl_generics #lain::traits::SerializedSize for #ident #ty_generics #where_clause {
            #[inline]
            fn serialized_size(&self) -> usize {
                use #lain::traits::SerializedSize;
                #lain::log::debug!("getting serialized size of {}", #ident_as_string);
                let size = #serialized_size_body;
                #lain::log::debug!("size of {} is 0x{:02X}", #ident_as_string, size);

                return size;
            }

            #[inline]
            fn min_nonzero_elements_size() -> usize {
                #min_nonzero_elements_body
            }
        }
    };

    let data = dummy::wrap_in_const("BINARYSERIALIZE", ident, impl_block);
    println!("{}", data);

    Ok(data)
}

fn binary_serialize_body(cont: &Container) -> TokenStream {
    match cont.data {
        Data::Enum(ref variants) if variants[0].style != Style::Unit => binary_serialize_enum(variants, &cont.attrs, &cont.ident),
        Data::Enum(ref variants) => binary_serialize_unit_enum(variants, &cont.attrs, &cont.ident),
        Data::Struct(Style::Struct, ref fields) | Data::Struct(Style::Tuple, ref fields) => binary_serialize_struct(fields, &cont.attrs, &cont.ident),
        Data::Struct(Style::Unit, ref fields) => TokenStream::new(),
    }
}

fn serialized_size_body(cont: &Container) -> (TokenStream, TokenStream) {
    match cont.data {
        Data::Enum(ref variants) if variants[0].style != Style::Unit => serialized_size_enum(variants, &cont.attrs, &cont.ident),
        Data::Enum(ref variants) => serialized_size_unit_enum(variants, &cont.attrs, &cont.ident),
        Data::Struct(Style::Struct, ref fields) | Data::Struct(Style::Tuple, ref fields) => serialized_size_struct(fields, &cont.attrs, &cont.ident),
        Data::Struct(Style::Unit, ref fields) => (quote!{0}, quote!{0}),
    }
}

fn binary_serialize_enum (
    variants: &[Variant],
    cattrs: &attr::Container,
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

fn binary_serialize_unit_enum(variants: &[Variant], cattrs: &attr::Container, cont_ident: &syn::Ident) -> TokenStream {
    quote! {
        <<#cont_ident as _lain::traits::ToPrimitive>::Output>::binary_serialize::<_, E>(&self.to_primitive(), buffer);
    }
}

fn binary_serialize_struct (
    fields: &[Field],
    cattrs: &attr::Container,
    cont_ident: &syn::Ident,
) -> TokenStream {
    let serializers = binary_serialize_struct_visitor(fields, cont_ident);

    quote! {
        let mut bitfield: u64 = 0;

        #(#serializers)*
    }
}

fn binary_serialize_struct_visitor(
    fields: &[Field],
    cont_ident: &syn::Ident,
) -> Vec<TokenStream> {
    fields
        .iter()
        .map(|field| {
            let (_field_ident, _field_ident_string, serializer) = field_serializer(field, "self.", false);

            quote! {
                #serializer
            }
        })
        .collect()
}

fn field_serializer(field: &Field, name_prefix: &'static str, is_destructured: bool) -> (TokenStream, String, TokenStream) {
    let ty = &field.ty;
    let field_ident = &field.member;
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

        let bitfield_type = field.attrs.bitfield_type().unwrap_or(&field.ty);

        let type_total_bits = if is_primitive_type(bitfield_type, "u8") {
            8
        } else if is_primitive_type(&field.ty, "u16") {
            16
        } else if is_primitive_type(&field.ty, "u32") {
            32
        } else if is_primitive_type(&field.ty, "u64") {
            64
        } else {
            panic!("got to field_serialize with an unsupported bitfield type. ensure that checks in ast code are correct");
        };

        let bitfield_value = if field.attrs.bitfield_type().is_some() {
            quote_spanned! {field.ty.span() => #value_ident.to_primitive()}
        } else {
            quote!{#value_ident}
        };

        let mut bitfield_setter = quote_spanned!{ field.ty.span() =>
            bitfield |= ((#bitfield_value as #bitfield_type & #bit_mask as #bitfield_type) << #bit_shift) as u64;
        };

        if bits + bit_shift == type_total_bits {
            bitfield_setter.extend(quote_spanned!{field.ty.span() => <#ty>::binary_serialize::<_, #endian>(&(bitfield as #ty), buffer);});
        }

        bitfield_setter
    } else {
        if let syn::Type::Array(ref a) = ty {
            // TODO: Change this once const generics are stabilized
            quote! {
                #value_ident.binary_serialize::<_, #endian>(buffer);
            }
        } else {
            quote! {
                <#ty>::binary_serialize::<_, #endian>(#borrow#value_ident, buffer);
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
                let (value_ident, field_ident_string, initializer) = field_serializer(field, "__field", true);
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
    cattrs: &attr::Container,
    cont_ident: &syn::Ident,
) ->  (TokenStream, TokenStream) {
    let match_arms = serialized_size_enum_visitor(variants, cont_ident, SerializedSizeVisitorType::SerializedSize);
    let nonzero_variants = serialized_size_enum_visitor(variants, cont_ident, SerializedSizeVisitorType::MinNonzeroElements);

    let serialized_size = quote! {
        match *self {
            #(#match_arms)*
        }
    };
    
    let min_nonzero = quote! {*[#(#nonzero_variants,)*].iter().min_by(|a, b| a.cmp(b)).unwrap()};

    (serialized_size, min_nonzero)
}

fn serialized_size_unit_enum(variants: &[Variant], cattrs: &attr::Container, cont_ident: &syn::Ident) -> (TokenStream, TokenStream) {
    let size = quote! {
        std::mem::size_of::<<#cont_ident as _lain::traits::ToPrimitive>::Output>()
    };

    (size.clone(), size)
}

fn serialized_size_struct (
    fields: &[Field],
    cattrs: &attr::Container,
    cont_ident: &syn::Ident,
) -> (TokenStream, TokenStream) {
    let serialized_size = serialized_size_struct_visitor(fields, cont_ident, SerializedSizeVisitorType::SerializedSize);

    let min_nonzero = serialized_size_struct_visitor(fields, cont_ident, SerializedSizeVisitorType::MinNonzeroElements);

    (quote! {0 #(+#serialized_size)* }, quote! { 0 #(+#min_nonzero)* })
}

fn serialized_size_struct_visitor(
    fields: &[Field],
    cont_ident: &syn::Ident,
    visitor_type: SerializedSizeVisitorType,
) -> Vec<TokenStream> {
    fields
        .iter()
        .map(|field| {
            let (_field_ident, _field_ident_string, serialized_size) = field_serialized_size(field, "self.", false, visitor_type);

            quote! {
                #serialized_size
            }
        })
        .collect()
}

fn field_serialized_size(field: &Field, name_prefix: &'static str, is_destructured: bool, visitor_type: SerializedSizeVisitorType) -> (TokenStream, String, TokenStream) {
    let ty = &field.ty;
    let field_ident = &field.member;
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

        let type_total_bits = if is_primitive_type(bitfield_type, "u8") {
            8
        } else if is_primitive_type(&field.ty, "u16") {
            16
        } else if is_primitive_type(&field.ty, "u32") {
            32
        } else if is_primitive_type(&field.ty, "u64") {
            64
        } else {
            panic!("got to field_serialize with an unsupported bitfield type. ensure that checks in ast code are correct");
        };

        let bitfield_value = if field.attrs.bitfield_type().is_some() {
            quote_spanned! {field.ty.span() => #value_ident.to_primitive()}
        } else {
            quote!{#borrow#value_ident}
        };

        // kind of a hack but only emit the size of the bitfield once we've reached
        // the last item in the bitfield
        if bits + bit_shift == type_total_bits {
            if visitor_type == SerializedSizeVisitorType::SerializedSize {
                quote!{_lain::traits::SerializedSize::serialized_size(#bitfield_value)}
            } else {
                quote!{<#bitfield_type>::min_nonzero_elements_size()}
            }
        } else {
            quote!{0}
        }
    } else {
        if visitor_type == SerializedSizeVisitorType::SerializedSize {
            quote! {
                _lain::traits::SerializedSize::serialized_size(#borrow#value_ident)
            }
        } else {
            quote! {
                <#ty>::min_nonzero_elements_size()
            }
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
                let (value_ident, field_ident_string, field_size) = field_serialized_size(field, "__field", true, visitor_type);
                field_identifiers.push(quote_spanned!{ field.member.span() => #value_ident });

                field_size
            })
            .collect();

            if visitor_type == SerializedSizeVisitorType::SerializedSize {
                quote! {
                    #full_ident(#(ref #field_identifiers,)*) => {
                        0 #(+#field_sizes)*
                    }
                }
            } else {
                quote! {
                    0 #(+#field_sizes)*
                }
            }
        })
        .collect();

    match_arms
}