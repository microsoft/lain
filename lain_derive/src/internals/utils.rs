use proc_macro2::TokenStream;

use quote::{quote, quote_spanned, ToTokens};

use syn::spanned::Spanned;
use syn::{IntSuffix, LitInt};
use syn::{Meta, NestedMeta};

use std::str::FromStr;

use crate::attr::*;

#[derive(Debug, PartialEq)]
pub enum PrimitiveType {
    None,
    Bool,
    Number,
}


pub(crate) struct FuzzerObjectStructField<'a> {
    pub field: &'a syn::Field,
    pub min: Option<TokenStream>,
    pub max: Option<TokenStream>,
    pub ignore: bool,
    pub user_initializer: Option<TokenStream>,
    pub ignore_chance: f64,
    pub is_bitfield: bool,
    pub weighted: Weighted,
}

pub(crate) fn is_primitive(ty: &str) -> PrimitiveType {
    match ty {
        "f32" | "f64" | "u8" | "i8" | "u16" | "i16" | "u32" | "i32" | "u64" | "i64" => {
            PrimitiveType::Number
        }
        "bool" => PrimitiveType::Bool,
        _ => PrimitiveType::None,
    }
}

pub(crate) fn parse_fields(fields: &syn::FieldsNamed) -> Vec<FuzzerObjectStructField> {
    fields
        .named
        .iter()
        .map(|f| {
            let mut field = FuzzerObjectStructField {
                field: f,
                min: None,
                max: None,
                ignore_chance: 0.0,
                ignore: false,
                user_initializer: None,
                is_bitfield: false,
                weighted: Weighted::None,
            };

            let _ty = &f.ty;

            let meta = f.attrs.iter().filter_map(get_fuzzer_metadata);
            for meta_items in meta {
                for meta_item in meta_items {
                    match meta_item {
                        NestedMeta::Meta(Meta::NameValue(ref m)) if m.ident == "weighted" => {
                            if let syn::Lit::Str(ref s) = m.lit {
                                field.weighted = match s.value().as_ref() {
                                    "min" => Weighted::Min,
                                    "max" => Weighted::Max,
                                    other => panic!("Unknown weighted value: {}", other),
                                };
                            }
                        }
                        NestedMeta::Meta(Meta::NameValue(ref m)) if m.ident == "min" => {
                            // we basically only parse out strings since we want to treat those as non-string TokenStreams
                            if let Ok(s) = get_lit_str(&m.lit) {
                                let value = TokenStream::from_str(&s.value())
                                    .expect("invalid tokens for min");
                                field.min = Some(quote_spanned! {m.lit.span() => #value});
                            } else if let Ok(i) = get_lit_number(&m.lit) {
                                let int = LitInt::new(i.value(), IntSuffix::None, m.lit.span());
                                field.min = Some(quote_spanned! {m.lit.span() => #int});
                            } else {
                                let lit = &m.lit;
                                field.min = Some(quote_spanned! {m.lit.span() => #lit});
                            }
                        }
                        NestedMeta::Meta(Meta::NameValue(ref m)) if m.ident == "max" => {
                            if let Ok(s) = get_lit_str(&m.lit) {
                                let value = TokenStream::from_str(&s.value())
                                    .expect("invalid tokens for max");
                                field.max = Some(quote_spanned! {m.lit.span() => #value});
                            } else if let Ok(i) = get_lit_number(&m.lit) {
                                let int = LitInt::new(i.value(), IntSuffix::None, m.lit.span());
                                field.max = Some(quote_spanned! {m.lit.span() => #int});
                            } else {
                                let lit = &m.lit;
                                field.max = Some(quote_spanned! {m.lit.span() => #lit});
                            }
                        }
                        NestedMeta::Meta(Meta::NameValue(ref m)) if m.ident == "ignore" => {
                            if let Ok(s) = get_lit_bool(&m.lit) {
                                field.ignore = s.value;
                            }
                        }
                        NestedMeta::Meta(Meta::Word(ref ident)) if ident == "ignore" => {
                            field.ignore = true;
                        }
                        NestedMeta::Meta(Meta::NameValue(ref m)) if m.ident == "ignore_chance" => {
                            if let syn::Lit::Float(ref f) = m.lit {
                                field.ignore_chance = f.value() as f64;
                            } else {
                                panic!("ignore_chance field should be a f64");
                            }
                        }
                        NestedMeta::Meta(Meta::NameValue(ref m)) if m.ident == "initializer" => {
                            if let syn::Lit::Str(ref s) = m.lit {
                                field.user_initializer = Some(
                                    TokenStream::from_str(&s.value())
                                        .expect("invalid tokens for initializer"),
                                );
                            }
                        }
                        _ => continue,
                    }
                }
            }

            // parse out all of the details about min/max, etc. before we return
            // anything. we do this so that we can ensure we have all required
            // operations to do things in the mutator as well
            let meta = f.attrs.iter().filter_map(get_bitfield_metadata);
            if let Some(bitfield_meta) = get_bitfield_limits(meta) {
                field.is_bitfield = true;
                // TODO: we can provide better diagnostics if we instead use the span
                // of the nested meta items
                let min = LitInt::new(bitfield_meta.min, IntSuffix::None, f.span());
                let max = LitInt::new(bitfield_meta.max, IntSuffix::None, f.span());

                field.min = Some(quote! {#min});
                field.max = Some(quote! {#max});
            }

            field
        })
        .collect()
}
