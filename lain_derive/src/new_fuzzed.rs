use proc_macro2::TokenStream;
use quote::{quote, quote_spanned};
use std::str::FromStr;
use syn::spanned::Spanned;
use syn::{quote, quote_spanned};
use syn::{parse_macro_input, DeriveInput, Lit, NestedMeta};

use crate::internals::{Ctxt, Derive, attr};
use crate::internals::ast::{Container, Data, Field};
use crate::fragment::Fragment;

pub fn expand_new_fuzzed(input: &syn::DeriveInput) -> Result<TokenStream, Vec<syn::Error>> {
    let ctx = Ctxt::new();

    let cont = match Container::from_ast(&ctx, input, Derive::NewFuzzed) {
        Some(cont) => cont,
        None => return Err(ctx.check().unwrap_err()),
    };

    let ident = &cont.ident;
    let (impl_generics, ty_generics, where_clause) = input.generics.split_for_impl();

    let body = new_fuzzed_body(&cont);
    let lain = cont.attrs.lain_path();

    let impl_block = quote! {
        impl #impl_generics #lain::traits::NewFuzzed for #ident #ty_generics #where_clause {
            type RangeType = u8;

            fn new_fuzzed<R: #lain::rand::Rng>(mutator: &mut #lain::mutator::Mutator<R>, parent_constraints: Option<&#lain::types::Constraints<Self::RangeType>>) -> Self
            {
                #body
            }
        }
    };

    Ok(dummy::wrap_in_const(lain, "NEWFUZZED", ident, impl_block))
}

fn new_fuzzed_body(cont: &Container) -> Fragment {
    match cont.data {
        Data::Enum(ref variants) => new_fuzzed_enum(variants, &cont.attrs),
        Data::Struct(Style::Struct, ref fields) => new_fuzzed_struct(fields, &cont.attrs, &cont.ident),
        Data::Struct(Style::Tuple, ref fields) => new_fuzzed_tuple_struct(fields, &cont.attrs),
        Data::Struct(Style::Unit, ref fields) => new_fuzzed_unit_struct(fields, &cont.attrs),
    }
}

fn new_fuzzed_enum (
    fields: &[Field],
    cattrs: &attr::Container,
) -> Fragment {
        let serialize_fields = serialize_struct_visitor(fields, params, false, &StructTrait::SerializeStruct);

    let type_name = cattrs.name().serialize_name();

    let tag_field = serialize_struct_tag_field(cattrs, &StructTrait::SerializeStruct);
    let tag_field_exists = !tag_field.is_empty();

    let mut serialized_fields = fields
        .iter()
        .filter(|&field| !field.attrs.skip_serializing())
        .peekable();

    let let_mut = mut_if(serialized_fields.peek().is_some() || tag_field_exists);

    let len = serialized_fields
        .map(|field| match field.attrs.skip_serializing_if() {
            None => quote!(1),
            Some(path) => {
                let field_expr = get_member(params, field, &field.member);
                quote!(if #path(#field_expr) { 0 } else { 1 })
            }
        })
        .fold(
            quote!(#tag_field_exists as usize),
            |sum, expr| quote!(#sum + #expr),
        );

    quote_block! {
        let #let_mut __serde_state = try!(_serde::Serializer::serialize_struct(__serializer, #type_name, #len));
        #tag_field
        #(#serialize_fields)*
        _serde::ser::SerializeStruct::end(__serde_state)
    }
}

fn new_fuzzed_struct (
    fields: &[Field],
    cattrs: &attr::Container,
    cont_ident: &syn::Ident,
) -> Fragment {
    let new_fuzzed_fields = new_fuzzed_struct_visitor(fields, false, cont_ident);
}

fn new_fuzzed_tuple_struct (
    fields: &[Field],
    cattrs: &attr::Container,
) -> Fragment {

}

fn new_fuzzed_unit_struct (
    fields: &[Field],
    cattrs: &attr::Container,
) -> Fragment {

}

fn new_fuzzed_struct_visitor(
    fields: &[Field],
    is_enum: bool,
    cont_ident: &syn::Ident,
) -> Vec<TokenStream> {
    fields
        .iter()
        .map(|field| {
            let default_constraints = field_constraints(field);
            let ty = &field.ty;
            let field_ident = &field.member;
            let field_ident_string = match field.member{
                syn::Member::Named(ref ident) => ident.to_string(),
                syn::Member::Unnamed(ref idx) => idx.to_string(),
            };

            let default_initializer = quote! {
                <#ty>::new_fuzzed(mutator, constraints.as_ref())
            };

            let initializer = if field.attrs.ignore() {
                quote! {
                    let value = <#ty>::default();
                }
            } else if let Some(chance) = field.attrs.ignore_chance() {
                quote_spanned! { ty.span()
                    let value = if mutator.gen_chance(#chance) {
                        <#ty>::default()
                    } else {
                        #default_initializer
                    };
                }
            } else if let Some(initializer) = field.attrs.initializer() {
                quote_spanned! { initializer.span() =>
                    let value = #initializer;
                }
            } else {
                quote_spanned! { ty.span() =>
                    let value = #default_initializer
                }
            };

            quote! {
                #default_constraints

                #initializer

                if <#ty>::is_variable_size() {
                    if let Some(ref mut max_size) = max_size {
                        if value.serialized_size() > *max_size {
                            warn!("Max size provided to {} object is likely smaller than min object size", #field_ident_string);
                            *max_size = 0;
                        } else {
                            *max_size -= value.serialized_size();
                        }
                    }
                }

                let field_offset = _lain::field_offset::offset_of!(#cont_ident => #field_ident).get_byte_offset() as isize;

                unsafe {
                    let field_ptr = (uninit_struct_ptr as *mut u8).offset(field_offset) as *mut #ty;

                    std::ptr::write(field_ptr, value);
                }
            }
        })
        .collect()
}

fn field_constraints(field: &Field) -> TokenStream {
    let attrs = &field.attrs;
    if attrs.min().is_some() || attrs.max().is_some() || attrs.bits().is_some() {
        if let Some(bits) = attrs.bits() {
            quote! {
                let constraints = parent_constraints.and_then(|c| {
                    if c.max_size.is_none() {
                        None
                    } else {
                        c.clone();
                        c.max_size = max_size;
                    }
                });
            }
        } else {
            let min = attrs.min();
            let max = attrs.max();
            let weight_to = attrs.weight_to();
            quote! {
                let constraints = parent_constraints.and_then(|c| {
                    if c.max_size.is_none() {
                        None
                    } else {
                        c.clone();
                        c.min = #min;
                        c.max = #max;
                        c.weighted = #weight_to;
                        c.max_size = max_size;
                    }
                });
            }
        }
    } else {
        quote! {
            let constraints = parent_constraints.and_then(|c| {
                if c.max_size.is_none() {
                    None
                } else {
                    c.clone();
                    c.max_size = max_size;
                }
            });
        }
    }
}