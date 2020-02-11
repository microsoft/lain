use proc_macro2::{Span, TokenStream};
use std::str::FromStr;
use syn::spanned::Spanned;
use quote::{quote, quote_spanned, ToTokens};

use crate::internals::{Ctxt, Derive, attr};
use crate::internals::ast::{Container, Data, Field, Variant, Style};
use crate::dummy;

pub fn expand_mutatable(input: &syn::DeriveInput) -> Result<TokenStream, Vec<syn::Error>> {
    let ctx = Ctxt::new();

    let cont = match Container::from_ast(&ctx, input, Derive::Mutatable) {
        Some(cont) => cont,
        None => return Err(ctx.check().unwrap_err()),
    };

    ctx.check()?;

    let ident = &cont.ident;
    let (impl_generics, ty_generics, where_clause) = input.generics.split_for_impl();

    let body = mutatable_body(&cont);
    let lain = cont.attrs.lain_path();

    let impl_block = quote! {
        #[automatically_derived]
        impl #impl_generics #lain::traits::Mutatable for #ident #ty_generics #where_clause {
            // structs always have a RangeType of u8 since they shouldn't
            // really use the min/max
            type RangeType = u8;

            fn mutate<R: #lain::rand::Rng>(&mut self, mutator: &mut #lain::mutator::Mutator<R>, parent_constraints: Option<&#lain::types::Constraints<Self::RangeType>>)
            {
                #body
            }
        }
    };

    let data = dummy::wrap_in_const("MUTATABLE", ident, impl_block);

    Ok(data)
}

pub fn expand_new_fuzzed(input: &syn::DeriveInput) -> Result<TokenStream, Vec<syn::Error>> {
    let ctx = Ctxt::new();

    let cont = match Container::from_ast(&ctx, input, Derive::BinarySerialize) {
        Some(cont) => cont,
        None => return Err(ctx.check().unwrap_err()),
    };

    ctx.check()?;

    let ident = &cont.ident;
    let (impl_generics, ty_generics, where_clause) = input.generics.split_for_impl();

    let body = new_fuzzed_body(&cont);
    let lain = cont.attrs.lain_path();

    let impl_block = quote! {
        #[automatically_derived]
        impl #impl_generics #lain::traits::NewFuzzed for #ident #ty_generics #where_clause {
            type RangeType = u8;

            // structs always have a RangeType of u8 since they shouldn't
            // really use the min/max
            fn new_fuzzed<R: #lain::rand::Rng>(mutator: &mut #lain::mutator::Mutator<R>, parent_constraints: Option<&#lain::types::Constraints<Self::RangeType>>) -> Self
            {
                #body
            }
        }
    };

    let data = dummy::wrap_in_const("NEWFUZZED", ident, impl_block);

    Ok(data)
}

fn mutatable_body(cont: &Container) -> TokenStream {
    match cont.data {
        Data::Enum(ref variants) if variants[0].style != Style::Unit => mutatable_enum(variants, &cont.ident),
        Data::Enum(ref variants) => mutatable_unit_enum(variants, &cont.ident),
        Data::Struct(Style::Struct, ref fields) | Data::Struct(Style::Tuple, ref fields) => mutatable_struct(fields),
        Data::Struct(Style::Unit, ref _fields) => TokenStream::new(),
    }
}

fn mutatable_enum (
    variants: &[Variant],
    cont_ident: &syn::Ident,
) ->  TokenStream {
    let constraints_prelude = constraints_prelude();
    let match_arms = mutatable_enum_visitor(variants, cont_ident);

    if match_arms.is_empty() {
        return TokenStream::new();
    }

    quote! {
        if mutator.mode() == _lain::mutator::MutatorMode::Havoc {
            *self = Self::new_fuzzed(mutator, parent_constraints);
            return;
        }
        
        #constraints_prelude

        match *self {
            #(#match_arms)*
            _ => { /* these are ignored */ },
        }
    }
}

fn mutatable_unit_enum(variants: &[Variant], cont_ident: &syn::Ident) -> TokenStream {
    let (weights, variant_tokens) = mutatable_unit_enum_visitor(variants, cont_ident);
    let variant_count = variant_tokens.len();

    if variant_tokens.is_empty() {
        return TokenStream::new();
    }

    quote! {
        use _lain::rand::seq::SliceRandom;
        use _lain::rand::distributions::Distribution;

        static options: [#cont_ident; #variant_count] = [#(#variant_tokens,)*];

        static weights: [u64; #variant_count] = [#(#weights,)*];

        _lain::lazy_static::lazy_static! {
            static ref dist: _lain::rand::distributions::WeightedIndex<u64> =
                _lain::rand::distributions::WeightedIndex::new(weights.iter()).unwrap();
        }

        let idx: usize = dist.sample(&mut mutator.rng);
        *self = options[idx]
    }
}

fn mutatable_struct (
    fields: &[Field],
) -> TokenStream {
    let mutators = mutatable_struct_visitor(fields);
    let prelude = constraints_prelude();

    if mutators.is_empty() {
        return TokenStream::new();
    }

    quote! {
        #prelude

        #(#mutators)*

        if mutator.should_fixup() {
            self.fixup(mutator);
        }
    }
}

fn mutatable_unit_enum_visitor(
    variants: &[Variant],
    cont_ident: &syn::Ident,
) -> (Vec<u64>, Vec<TokenStream>) {
    let mut weights = vec![];

    let variants = variants.iter().filter_map(|variant| {
        if variant.attrs.ignore() {
            None
        } else {
            let variant_ident = &variant.ident;
            weights.push(variant.attrs.weight().unwrap_or(1));
            Some(quote!{#cont_ident::#variant_ident})
        }
    })
    .collect();

    (weights, variants)
}

fn mutatable_enum_visitor(
    variants: &[Variant],
    cont_ident: &syn::Ident,
) -> Vec<TokenStream> {
    let match_arms = variants
        .iter()
        .filter_map(|variant| {
            if variant.attrs.ignore() {
                return None;
            }

            let variant_ident = &variant.ident;
            let full_ident = quote!{#cont_ident::#variant_ident};
            let mut field_identifiers = vec![];

            let field_mutators: Vec<TokenStream> = variant.fields.iter().map(|field| {
                let (value_ident, _field_ident_string, initializer) = field_mutator(field, "__field", true);
                field_identifiers.push(quote_spanned!{ field.member.span() => #value_ident });

                initializer
            })
            .collect();

            let match_arm = quote! {
                #full_ident(#(ref mut #field_identifiers,)*) => {
                    #(#field_mutators)*
                }
                _ => { /* these are ignored */ }
            };

            Some(match_arm)
        })
        .collect();

    match_arms
}

fn mutatable_struct_visitor(
    fields: &[Field],
) -> Vec<TokenStream> {
    fields
        .iter()
        .map(|field| {
            let (_field_ident, _field_ident_string, initializer) = field_mutator(field, "self.", false);

            quote! {
                #initializer
            }
        })
        .collect()
}

fn new_fuzzed_body(cont: &Container) -> TokenStream {
    match cont.data {
        Data::Enum(ref variants) if variants[0].style != Style::Unit => new_fuzzed_enum(variants, &cont.ident),
        Data::Enum(ref variants) => new_fuzzed_unit_enum(variants, &cont.ident),
        Data::Struct(Style::Struct, ref fields) | Data::Struct(Style::Tuple, ref fields) => new_fuzzed_struct(fields, &cont.ident),
        Data::Struct(Style::Unit, ref _fields) => new_fuzzed_unit_struct(&cont.ident),
    }
}

fn new_fuzzed_enum (
    variants: &[Variant],
    cont_ident: &syn::Ident,
) ->  TokenStream {
    let constraints_prelude = constraints_prelude();
    let (weights, new_fuzzed_fields) = new_fuzzed_enum_visitor(variants, cont_ident);
    let variant_count = new_fuzzed_fields.len();

    if new_fuzzed_fields.is_empty() {
        return quote!{Default::default()};
    }

    let mut match_arms = vec![];

    for (i, variant) in new_fuzzed_fields.iter().enumerate() {
        match_arms.push(quote! {
            #i => {
                #variant

                if mutator.should_fixup() {
                    value.fixup(mutator);
                }

                value
            }
        });
    }

    quote! {
        use _lain::rand::seq::SliceRandom;
        use _lain::rand::distributions::Distribution;

        static weights: [u64; #variant_count] = [#(#weights,)*];

        _lain::lazy_static::lazy_static! {
            static ref dist: _lain::rand::distributions::WeightedIndex<u64> =
                _lain::rand::distributions::WeightedIndex::new(weights.iter()).unwrap();
        }

        #constraints_prelude

        let idx: usize = dist.sample(&mut mutator.rng);
        match idx {
            #(#match_arms)*
            _ => unreachable!(),
        }
    }
}

fn new_fuzzed_unit_enum(variants: &[Variant], cont_ident: &syn::Ident) -> TokenStream {
    let (weights, variant_tokens) = new_fuzzed_unit_enum_visitor(variants, cont_ident);

    if variant_tokens.is_empty() {
        return quote!{Default::default()};
    }

    let variant_count = variant_tokens.len();


    quote! {
        use _lain::rand::seq::SliceRandom;
        use _lain::rand::distributions::Distribution;

        static options: [#cont_ident; #variant_count] = [#(#variant_tokens,)*];

        static weights: [u64; #variant_count] = [#(#weights,)*];

        _lain::lazy_static::lazy_static! {
            static ref dist: _lain::rand::distributions::WeightedIndex<u64> =
                _lain::rand::distributions::WeightedIndex::new(weights.iter()).unwrap();
        }

        let idx: usize = dist.sample(&mut mutator.rng);
        options[idx]
    }
}

fn new_fuzzed_struct (
    fields: &[Field],
    cont_ident: &syn::Ident,
) -> TokenStream {
    let initializers = new_fuzzed_struct_visitor(fields, cont_ident);
    let prelude = constraints_prelude();

    let len = initializers.len();

    let mut match_arms = vec![];

    for (i, initializer) in initializers.iter().enumerate() {
        match_arms.push(quote! {
            #i => {
                #initializer
            }
        });
    }

    quote! {
        use _lain::rand::seq::index::sample;

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
    }
}

fn new_fuzzed_unit_struct (
    cont_ident: &syn::Ident,
) -> TokenStream {
    quote! {
        #cont_ident
    }
}

fn new_fuzzed_struct_visitor(
    fields: &[Field],
    cont_ident: &syn::Ident,
) -> Vec<TokenStream> {
    fields
        .iter()
        .map(|field| {
            let (field_ident, _field_ident_string, initializer) = field_initializer(field, "self");
            let ty = &field.ty;
            let member = &field.member;

            quote! {
                #initializer

                let field_offset = _lain::field_offset::offset_of!(#cont_ident => #member).get_byte_offset() as isize;

                unsafe {
                    let field_ptr = (uninit_struct_ptr as *mut u8).offset(field_offset) as *mut #ty;

                    std::ptr::write(field_ptr, #field_ident);
                }
            }
        })
        .collect()
}

fn struct_field_constraints(field: &Field, for_mutation: bool) -> TokenStream {
    let attrs = &field.attrs;
    if !for_mutation {
        if attrs.ignore() || (attrs.initializer().is_some() && !attrs.ignore_chance().is_some()) {
            return TokenStream::new(); 
        }
    }

    if attrs.min().is_some() || attrs.max().is_some() || attrs.bits().is_some() {
        let min: TokenStream;
        let max: TokenStream;

        if let Some(bits) = attrs.bits() {
            // TODO: maybe refactor attributes so that they can retain original span
            let bitfield_max = syn::LitInt::new(2_u64.pow(bits as u32), syn::IntSuffix::None, Span::call_site()); 
            max = quote!{Some(#bitfield_max)};
            min = quote!{Some(0)};
        } else {
            min = option_to_tokens(attrs.min());
            max = option_to_tokens(attrs.max());
        }

        let weight_to = attrs.weight_to().unwrap_or(&attr::WeightTo::None);
        quote! {
            let mut constraints = Constraints::new();
            constraints.min = #min;
            constraints.max = #max;
            constraints.weighted = #weight_to;
            constraints.max_size = max_size;
            constraints.base_object_size_accounted_for = true;
            let constraints = Some(constraints);
        }
    } else {
        quote! {
            let constraints = max_size.as_ref().and_then(|m| {
                let mut c = Constraints::new();
                c.base_object_size_accounted_for = true;
                c.max_size(*m);
                Some(c)
            });
        }
    }
}

fn option_to_tokens<T: ToTokens + Spanned>(opt: Option<&T>) -> TokenStream {
    opt.map_or_else(|| quote!{None}, |o| quote_spanned!{opt.span() => Some(#o)})
}

fn field_initializer(field: &Field, name_prefix: &'static str) -> (TokenStream, String, TokenStream) {
    let default_constraints = struct_field_constraints(field, false);
    let ty = &field.ty;
    let field_ident_string = match field.member{
        syn::Member::Named(ref ident) => ident.to_string(),
        syn::Member::Unnamed(ref idx) => idx.index.to_string(),
    };

    let value_ident = TokenStream::from_str(&format!("{}{}", name_prefix, field_ident_string)).unwrap();

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

    let inc_max_size = increment_max_size(&field, &value_ident);
    let initializer = quote! {
        #default_constraints 

        #initializer

        #inc_max_size
    };

    (value_ident, field_ident_string, initializer)
}

fn increment_max_size(field: &Field, value_ident: &TokenStream) -> TokenStream {
    let ty = field.ty;
    let _field_ident_string = match field.member {
        syn::Member::Named(ref ident) => ident.to_string(),
        syn::Member::Unnamed(ref idx) => idx.index.to_string(),
    };

    let zero_tokens = TokenStream::from_str("0").unwrap();
    let field_min_items = field.attrs.min().unwrap_or(&zero_tokens);
    let ty_size = quote!{
        ((<#ty>::min_nonzero_elements_size() * #field_min_items) as isize)
    };

    quote! {
        if <#ty>::is_variable_size() {
            if let Some(ref mut max_size) = max_size {
                // we only subtract off the difference between the object's allocated size
                // and its min size.
                let size_delta = (#value_ident.serialized_size() as isize) - #ty_size;

                // size_delta might be negative in the event that the mutator ignored
                // the min bound
                *max_size = ((*max_size as isize) - size_delta) as usize;
            }
        }
    }
}

fn field_mutator(field: &Field, name_prefix: &'static str, is_destructured: bool) -> (TokenStream, String, TokenStream) {
    let default_constraints = struct_field_constraints(field, true);
    let ty = &field.ty;
    let field_ident_string = match field.member{
        syn::Member::Named(ref ident) => ident.to_string(),
        syn::Member::Unnamed(ref idx) => idx.index.to_string(),
    };

    let value_ident = TokenStream::from_str(&format!("{}{}", name_prefix, field_ident_string)).unwrap();
    let borrow = if is_destructured {
        TokenStream::new()
    } else {
        quote!{&mut}
    };

    let mutator_stmts = quote! {
        <#ty>::mutate(#borrow #value_ident, mutator, constraints.as_ref());

        if mutator.should_early_bail_mutation() {
            if mutator.should_fixup() {
                <#ty>::fixup(#borrow #value_ident, mutator);
            }

            return;
        }
    };

    let inc_max_size = increment_max_size(&field, &value_ident);

    let initializer = quote! {
        #default_constraints 

        #mutator_stmts

        #inc_max_size
    };

    (value_ident, field_ident_string, initializer)
}

fn new_fuzzed_unit_enum_visitor(
    variants: &[Variant],
    cont_ident: &syn::Ident,
) -> (Vec<u64>, Vec<TokenStream>) {
    let mut weights = vec![];

    let variants = variants.iter().filter_map(|variant| {
        if variant.attrs.ignore() {
            None
        } else {
            let variant_ident = &variant.ident;
            weights.push(variant.attrs.weight().unwrap_or(1));
            Some(quote!{#cont_ident::#variant_ident})
        }
    })
    .collect();

    (weights, variants)
}

fn new_fuzzed_enum_visitor(
    variants: &[Variant],
    cont_ident: &syn::Ident,
) -> (Vec<u64>, Vec<TokenStream>) {
    let mut weights = vec![];
    let initializers = variants
        .iter()
        .filter_map(|variant| {
            if variant.attrs.ignore() {
                return None;
            }

            let variant_ident = &variant.ident;
            let full_ident = quote!{#cont_ident::#variant_ident};
            let mut field_identifiers = vec![];

            let field_initializers: Vec<TokenStream> = variant.fields.iter().map(|field| {
                let (value_ident, _field_ident_string, initializer) = field_initializer(field, "__field");
                field_identifiers.push(quote_spanned!{ field.member.span() => #value_ident });

                initializer
            })
            .collect();

            let initializer = quote! {
                #(#field_initializers)*

                let mut value = #full_ident(#(#field_identifiers,)*);
            };

            weights.push(variant.attrs.weight().unwrap_or(1));

            Some(initializer)
        })
        .collect();

    (weights, initializers)
}

fn constraints_prelude() -> TokenStream {
    quote! {
        // Make a copy of the constraints that will remain immutable for
        // this function. Here we ensure that the base size of this object has
        // been accounted for by the caller, which may be an object containing this.
        let parent_constraints = parent_constraints.and_then(|c| {
            let mut c = c.clone();
            c.account_for_base_object_size::<Self>();

            Some(c)
        });

        let mut max_size = parent_constraints.as_ref().and_then(|c| c.max_size);
        if let Some(max) = max_size {
            max_size = Some(std::cmp::min(Self::min_nonzero_elements_size(), max));
        }
    }
}
