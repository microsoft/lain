use proc_macro2::TokenStream;
use std::str::FromStr;
use syn::spanned::Spanned;
use syn::{parse_macro_input, DeriveInput, Lit, NestedMeta};
use quote::{quote, quote_spanned};

use crate::internals::{Ctxt, Derive, attr};
use crate::internals::ast::{Container, Data, Field, Variant, Style};
use crate::fragment::{Fragment, Match, Stmts, Expr};
use crate::dummy;

pub fn expand_new_fuzzed(input: &syn::DeriveInput) -> Result<TokenStream, Vec<syn::Error>> {
    let ctx = Ctxt::new();

    let cont = match Container::from_ast(&ctx, input, Derive::NewFuzzed) {
        Some(cont) => cont,
        None => return Err(ctx.check().unwrap_err()),
    };

    let ident = &cont.ident;
    let (impl_generics, ty_generics, where_clause) = input.generics.split_for_impl();

    let body = Stmts(new_fuzzed_body(&cont));
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

    Ok(dummy::wrap_in_const("NEWFUZZED", ident, impl_block))
}

fn new_fuzzed_body(cont: &Container) -> Fragment {
    match cont.data {
        Data::Enum(ref variants) if variants[0].style != Style::Unit => new_fuzzed_enum(variants, &cont.attrs, &cont.ident),
        Data::Enum(ref variants) if variants[0].style == Style::Unit => new_fuzzed_unit_enum(variants, &cont.attrs, &cont.ident),
        Data::Struct(Style::Struct, ref fields) | Data::Struct(Style::Tuple, ref fields) => new_fuzzed_struct(fields, &cont.attrs, &cont.ident),
        Data::Struct(Style::Unit, ref fields) => new_fuzzed_unit_struct(fields, &cont.attrs, &cont.ident),
    }
}

fn new_fuzzed_enum (
    variants: &[Variant],
    cattrs: &attr::Container,
    cont_ident: &syn::Ident,
) -> Expr {
    let constraints_prelude = constraints_prelude();
    let (weights, new_fuzzed_fields) = new_fuzzed_enum_visitor(variants, cont_ident);
    let variant_count = new_fuzzed_fields.len();

    let mut match_arms = vec![];

    for (i, variant) in new_fuzzed_fields.iter().enumerate() {
        match_arms.push(Stmts(quote_expr! {
            #i => {
                #variant

                if mutator.should_fixup() {
                    value.fixup(mutator);
                }

                value
            }
        }));
    }

    Expr(quote_block! {
        use _lain::rand::seq::SliceRandom;

        static weights: [u64; #variant_count] = [#(#weights,)*];

        ::lain::lazy_static::lazy_static! {
            static ref dist: ::lain::rand::distributions::WeightedIndex<u64> =
                ::lain::rand::distributions::WeightedIndex::new(weights.iter()).unwrap();
        }

        let idx: usize = dist.sample(&mut mutator.rng);
        match idx {
            #(#match_arms)*
            _ => unreachable!(),
        }
    })
}

fn new_fuzzed_unit_enum(variants: &[Variant], cattrs: &attr::Container, cont_ident: &syn::Ident) -> Expr {
    let (weights, variant_tokens) = new_fuzzed_unit_enum_visitor(variants, cont_ident);
    let variant_count = variant_tokens.len();

    Expr(quote_block! {
        use _lain::rand::seq::SliceRandom;

        static options: [#cont_ident; #variant_count] = [#(#variant_tokens,)*];

        static weights: [u64; #variant_count] = [#(#weights,)*];

        ::lain::lazy_static::lazy_static! {
            static ref dist: ::lain::rand::distributions::WeightedIndex<u64> =
                ::lain::rand::distributions::WeightedIndex::new(weights.iter()).unwrap();
        }

        let idx: usize = dist.sample(&mut mutator.rng);
        options[idx]
    })
}

fn new_fuzzed_struct (
    fields: &[Field],
    cattrs: &attr::Container,
    cont_ident: &syn::Ident,
) -> Stmts {
    let initializers = new_fuzzed_struct_visitor(fields, cont_ident);
    let prelude = constraints_prelude();

    let len = initializers.len();

    let mut match_arms = vec![];

    for (i, initializer) in initializers.iter().enumerate() {
        match_arms.push(Stmts(quote_expr! {
            #i => {
                #initializer
            }
        }));
    }

    Stmts(quote_block! {
        #prelude

        let mut uninit_struct = std::mem::MaybeUninit::<#cont_ident>::uninit();
        let uninit_struct_ptr = uninit_struct.as_mut_ptr();

        if Self::is_variable_size() {
            // this makes for ugly code generation, but better perf
            for i in sample(&mut mutator.rng, #len, #len).iter() {
                match i {
                    #(#match_arms)*
                    _ => unreachable!(),
                }
            }
        } else {
            #(#initializers)*
        }

        let mut initialized_struct = unsafe { uninit_struct.assume_init() };

        if mutator.should_fixup() {
            initialized_struct.fixup(mutator);
        }

        initialized_struct
    })
}

fn new_fuzzed_unit_struct (
    fields: &[Field],
    cattrs: &attr::Container,
    cont_ident: &syn::Ident,
) -> Fragment {
    quote_expr! {
        #cont_ident
    }
}

fn new_fuzzed_struct_visitor(
    fields: &[Field],
    cont_ident: &syn::Ident,
) -> Vec<Fragment> {
    fields
        .iter()
        .map(|field| {
            let (field_ident, field_ident_string, initializer) = field_initializer(field, "self_");
            let ty = &field.ty;

            quote_block! {
                #initializer

                let field_offset = _lain::field_offset::offset_of!(#cont_ident => #field_ident).get_byte_offset() as isize;

                unsafe {
                    let field_ptr = (uninit_struct_ptr as *mut u8).offset(field_offset) as *mut #ty;

                    std::ptr::write(field_ptr, value);
                }
            }
        })
        .collect()
}

fn struct_field_constraints(field: &Field) -> Fragment {
    let attrs = &field.attrs;
    if attrs.min().is_some() || attrs.max().is_some() || attrs.bits().is_some() {
        if let Some(bits) = attrs.bits() {
            quote_expr! {
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
            quote_expr! {
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
        quote_expr! {
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

fn field_initializer(field: &Field, name_prefix: &'static str) -> (syn::Ident, String, Fragment) {
    let default_constraints = struct_field_constraints(field);
    let ty = &field.ty;
    let field_ident = &field.member;
    let field_ident_string = match field.member{
        syn::Member::Named(ref ident) => ident.to_string(),
        syn::Member::Unnamed(ref idx) => idx.index.to_string(),
    };

    let value_ident = syn::Ident::new(&format!("{}_{}", name_prefix, field_ident_string), field.original.span());

    let default_initializer = quote! {
        <#ty>::new_fuzzed(mutator, constraints.as_ref())
    };

    let initializer = if field.attrs.ignore() {
        quote! {
            let #value_ident = <#ty>::default();
        }
    } else if let Some(chance) = field.attrs.ignore_chance() {
        quote_spanned! { ty.span() =>
            let #value_ident = if mutator.gen_chance(#chance) {
                <#ty>::default()
            } else {
                #default_initializer
            };
        }
    } else if let Some(initializer) = field.attrs.initializer() {
        quote_spanned! { initializer.span() =>
            let #value_ident = #initializer;
        }
    } else {
        quote_spanned! { ty.span() =>
            let #value_ident = #default_initializer;
        }
    };

    let initializer = quote_block! {
        #default_constraints 

        #initializer
        if <#ty>::is_variable_size() {
            if let Some(ref mut max_size) = max_size {
                if #value_ident.serialized_size() > *max_size {
                    warn!("Max size provided to {} object is likely smaller than min object size", #field_ident_string);
                    *max_size = 0;
                } else {
                    *max_size -= value.serialized_size();
                }
            }
        }
    };

    (value_ident, field_ident_string, initializer)
}

fn new_fuzzed_unit_enum_visitor(
    variants: &[Variant],
    cont_ident: &syn::Ident,
) -> (Vec<u64>, Vec<Fragment>) {
    let mut weights = vec![];

    let variants = variants.iter().filter_map(|variant| {
        if variant.attrs.ignore() {
            None
        } else {
            let variant_ident = &variant.ident;
            weights.push(variant.attrs.weight().unwrap_or(1));
            Some(quote_expr!{#cont_ident::#variant_ident})
        }
    })
    .collect();

    (weights, variants)
}

fn new_fuzzed_enum_visitor(
    variants: &[Variant],
    cont_ident: &syn::Ident,
) -> (Vec<u64>, Vec<Fragment>) {
    let mut weights = vec![];
    let initializers = variants
        .iter()
        .filter_map(|variant| {
            if variant.attrs.ignore() {
                return None;
            }

            let variant_ident = variant.ident;
            let full_ident = quote!{#cont_ident::#variant_ident};
            let variant_identifiers = vec![];

            let field_initializers = variant.fields.iter().map(|field| {
                let (value_ident, field_ident_string, initializer) = field_initializer(field, "__field");
                let member = &field.member;
                variant_identifiers.push(quote_spanned!{ field.member.span() => #member: #value_ident });

                quote_block! {
                    #default_constraints

                    #initializer
                }
            })
            .collect();

            let initializer = quote_block! {
                #(#field_initializers)*

                let value = #full_ident(#(#field_identifiers,)*);
            };

            weights.push(variant.attrs.weight().unwrap_or(1));

            Some(initializer)
        })
        .collect();

    (weights, initializers)
}

fn constraints_prelude() -> Fragment {
    quote_block! {
        // Make a copy of the constraints that will remain immutable for
        // this function. Here we ensure that the base size of this object has
        // been accounted for by the caller, which may be an object containing this.
        let parent_constraints = parent_constraints.and_then(|c| {
            let mut c = c.clone();
            //c.account_for_base_object_size::<Self>();

            Some(c)
        });

        let mut max_size = parent_constraints.as_ref().and_then(|c| c.max_size);
    }
}