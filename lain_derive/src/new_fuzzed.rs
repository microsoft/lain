extern crate proc_macro;

use crate::utils::*;
use proc_macro2::TokenStream;
use quote::{quote, quote_spanned};
use std::str::FromStr;
use syn::spanned::Spanned;
use syn::{parse_macro_input, Data, DeriveInput, Lit, NestedMeta};

use crate::attr::{get_attribute_metadata, get_fuzzer_metadata, get_lit_bool};

pub(crate) fn new_fuzzed_helper(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let input = parse_macro_input!(input as DeriveInput);

    let name = input.ident;
    let (impl_generics, ty_generics, where_clause) = input.generics.split_for_impl();

    let method_body: TokenStream;

    match input.data {
        Data::Enum(ref data) => {
            /// This struct represents an enum variant with parsed attributes
            struct Variant {
                full_ident: TokenStream,
                initializer: TokenStream,
                weight: u64,
                ignore: bool,
            }

            let mut variants = Vec::new();
            let mut enum_contains_items = false;

            for variant in &data.variants {
                let ident = &variant.ident;
                // This will look like EnumName::VariantName
                let full_ident = TokenStream::from_str(&format!(
                    "{}::{}",
                    &name.to_string(),
                    &ident.to_string()
                ))
                .unwrap();

                let mut variant_meta = Variant {
                    full_ident: full_ident.clone(),
                    initializer: TokenStream::new(),
                    weight: 1,
                    ignore: false,
                };

                // Parse the attributes
                let meta = variant.attrs.iter().filter_map(get_weighted_metadata);
                for meta_items in meta {
                    for meta_item in meta_items {
                        match meta_item {
                            NestedMeta::Literal(Lit::Int(ref i)) => {
                                variant_meta.weight = i.value();
                            }
                            _ => panic!("expected a literal int for #[weighted] attribute"),
                        }
                    }
                }

                let meta = variant.attrs.iter().filter_map(get_fuzzer_metadata);
                for meta_items in meta {
                    for meta_item in meta_items {
                        match meta_item {
                            syn::NestedMeta::Meta(syn::Meta::NameValue(ref m))
                                if m.ident == "ignore" =>
                            {
                                if let Ok(s) = get_lit_bool(&m.lit) {
                                    variant_meta.ignore = s.value;
                                }
                            }
                            _ => {}
                        }
                    }
                }

                // If we're supposed to ignore this attribute just continue the loop
                // before we try to build a match branch for this and add it to our known
                // variants. For our purposes, we now pretend like this thing doesn't exist
                if variant_meta.ignore {
                    continue;
                }

                match variant.fields {
                    // Unnamed fields look like:
                    // enum E {
                    //      Foo(X),
                    //      Bar(Y),
                    // }
                    syn::Fields::Unnamed(ref fields) => {
                        enum_contains_items = true;
                        let mut parameters = TokenStream::new();
                        let mut initializer = TokenStream::new();
                        let mut variant_sizes: Vec<TokenStream> = Vec::new();

                        // For each unnamed field we'll generate a placeholder
                        // name of the form field_N where N is its index
                        for (i, ref unnamed) in fields.unnamed.iter().enumerate() {
                            let field_type = &unnamed.ty;
                            let field_span = unnamed.span();
                            let ident_string = format!("field_{}", i);
                            let ident = TokenStream::from_str(&ident_string).unwrap();
                            
                            variant_sizes.push(quote_spanned!{field_span => <#field_type>::min_nonzero_element_size()});

                            initializer.extend(quote_spanned! { field_span =>
                                let mut #ident: #field_type = if let Some(ref constraints) = parent_constraints {
                                    let mut new_constraints = ::lain::types::Constraints::new();
                                    new_constraints.base_object_size_accounted_for = true;
                                    new_constraints.max_size = max_size;

                                    NewFuzzed::new_fuzzed(mutator, Some(&new_constraints))
                                } else {
                                    NewFuzzed::new_fuzzed(mutator, None)
                                };

                                if <#field_type>::is_variable_size() {
                                    if let Some(ref mut max_size) = max_size {
                                        if #ident.serialized_size() > *max_size {
                                            warn!("Max size provided to {} object is likely smaller than min object size", #ident_string);
                                            *max_size = 0;
                                        } else {
                                            *max_size -= #ident.serialized_size();
                                        }
                                    }
                                }
                            });

                            parameters.extend(quote! {#ident,});
                        }
                        let index = variants.len();

                        // Finally, we can build the branch to generate this item. This will look like:
                        // 0 => {
                        //     let mut field_0: X = NewFuzzed::new_fuzzed(mutator, None);
                        //     return EnumName::VariantName(field_0);
                        // }
                        variant_meta.initializer = quote! {
                            #index => {
                                #initializer
                                let mut result = #full_ident(#parameters);
                                if mutator.should_fixup() {
                                    result.fixup(mutator);
                                }

                                return result;
                            },
                        };
                    }
                    syn::Fields::Unit => {
                        // do nothing -- this is a simple enum type like
                        // enum { Foo, Bar, Baz, }
                        // or enum { Foo = 1, Bar, Baz,}
                    }
                    _ => {
                        panic!("Named fields aren't supported :( This should be easy to add though")
                    }
                }

                variants.push(variant_meta);
            }

            // Double-check to ensure we have no variants that want to be ignored
            let variants: Vec<&Variant> = variants.iter().filter(|v| !v.ignore).collect();
            let variant_count = variants.len();
            let weights = variants.iter().map(|v| v.weight);

            // This is the new_fuzzed function's inner body if we have non-basic enum variants
            let inner_body = if enum_contains_items {
                let ty = name.to_string();
                let variant_initializers = variants.iter().map(|v| v.initializer.clone());

                //
                quote! {
                    // Make a copy of the constraints that will remain immutable for
                    // this function. Here we ensure that the base size of this object has
                    // been accounted for by the caller, which may be an object containing this.
                    let parent_constraints = parent_constraints.and_then(|c| {
                        let mut c = c.clone();
                        //c.account_for_base_object_size::<Self>();

                        Some(c)
                    });

                    let mut max_size = parent_constraints.as_ref().and_then(|c| c.max_size);

                    let num: usize = dist.sample(&mut mutator.rng);
                    match num {
                        #(#variant_initializers)*
                        i => {
                            panic!("got an invalid number ({} > {} for type {})when generating a new item. check the codegen min/max", #variant_count, i, #ty)
                        }
                    }
                }
            } else {
                // We have basic enum variants that are just numbers
                let variant_tokens = variants.iter().map(|v| v.full_ident.clone());
                quote! {
                    use ::lain::rand::seq::SliceRandom;

                    static options: [#name; #variant_count] = [#(#variant_tokens,)*];

                    *options.choose(&mut mutator.rng).unwrap()
                }
            };

            method_body = quote! {
                static weights: [u64; #variant_count] = [#(#weights,)*];

                ::lain::lazy_static::lazy_static! {
                    static ref dist: ::lain::rand::distributions::WeightedIndex<u64> =
                        ::lain::rand::distributions::WeightedIndex::new(weights.iter()).unwrap();
                }

                #inner_body
            };
        }
        Data::Struct(ref data) => {
            if let syn::Fields::Named(ref fields) = data.fields {
                let fields = parse_fields(&fields);
                method_body = gen_struct_new_fuzzed_impl(&name, &fields);
            } else {
                panic!("currently no support for unnamed fields for NewFuzzed");
            }
        }
        _ => panic!("NewFuzzed only supports enums and structs"),
    }

    let expanded = quote! {
        impl #impl_generics ::lain::traits::NewFuzzed for #name #ty_generics #where_clause {
            type RangeType = u8;

            fn new_fuzzed<R: ::lain::rand::Rng>(mutator: &mut ::lain::mutator::Mutator<R>, parent_constraints: Option<&::lain::types::Constraints<Self::RangeType>>) -> #name
            {
                #method_body
            }
        }
    };

    // Uncomment to dump the AST
    // println!("{}\n\n", expanded);

    proc_macro::TokenStream::from(expanded)
}

/// Gets the meta items for #[weight()] attributes
fn get_weighted_metadata(attr: &syn::Attribute) -> Option<Vec<syn::NestedMeta>> {
    get_attribute_metadata("weight", &attr)
}

fn gen_struct_new_fuzzed_impl(
    name: &syn::Ident,
    fields: &[FuzzerObjectStructField],
) -> TokenStream {
    let mut generate_arms = vec![];
    let mut generate_linear = vec![];

    for (i, f) in fields.iter().enumerate() {
        let span = f.field.span();
        let ty = &f.field.ty;

        let mut field_mutation_tokens = TokenStream::new();
        let ident = &f.field.ident;

        // If the field is ignored, return the default value
        if f.ignore {
            field_mutation_tokens.extend(quote_spanned! { span =>
                let value = <#ty>::default();
            });
        }
        // If the user supplied an initializer, use that
        else if let Some(ref initializer) = f.user_initializer {
            field_mutation_tokens.extend(quote_spanned! { span =>
                let value = #initializer;
            });
        } else {
            // Otherwise, we assume that the field implements NewFuzzed and
            // we generate that value here

            let weighted = &f.weighted;

            let default_constraints = if f.min.is_some() || f.max.is_some() {
                let min = f
                    .min
                    .as_ref()
                    .map(|v| quote! {Some(#v)})
                    .unwrap_or_else(|| quote! {None});
                let max = f
                    .max
                    .as_ref()
                    .map(|v| quote! {Some(#v)})
                    .unwrap_or_else(|| quote! {None});

                quote_spanned! { span =>
                    let mut constraints = Constraints::new();
                    constraints.min = #min;
                    constraints.max = #max;
                    constraints.max_size = max_size.clone();
                    constraints.weighted = #weighted;
                    constraints.base_object_size_accounted_for = true;

                    let constraints = Some(constraints);
                }
            } else {
                quote_spanned! { span =>
                    let constraints = max_size.and_then(|max|{
                        let mut c = ::lain::types::Constraints::new();
                        c.base_object_size_accounted_for = true;
                        c.max_size(max);

                        Some(c)
                    });
                }
            };

            field_mutation_tokens.extend(quote_spanned! { span =>
                #default_constraints
                let value = <#ty>::new_fuzzed(mutator, constraints.as_ref());
            });
        }

        let ident_string = ident.as_ref().unwrap().to_string();
        field_mutation_tokens.extend(quote! {
            if <#ty>::is_variable_size() {
                if let Some(ref mut max_size) = max_size {
                    if value.serialized_size() > *max_size {
                        warn!("Max size provided to {} object is likely smaller than min object size", #ident_string);
                        *max_size = 0;
                    } else {
                        *max_size -= value.serialized_size();
                    }
                }
            }

            let field_offset = ::lain::field_offset::offset_of!(#name => #ident).get_byte_offset() as isize;

            unsafe {
                let field_ptr = (uninit_struct_ptr as *mut u8).offset(field_offset) as *mut #ty;

                std::ptr::write(field_ptr, value);
            }
        });

        generate_linear.push(field_mutation_tokens.clone());

        generate_arms.push(quote! {
            #i => {
                #field_mutation_tokens
            }
        });
    }

    let generate_fields_count = generate_arms.len();

    quote! {
        use std::any::Any;
        use ::lain::rand::seq::index::sample;

        // Make a copy of the constraints that will remain immutable for
        // this function. Here we ensure that the base size of this object has
        // been accounted for by the caller, which may be an object containing this.
        let parent_constraints = parent_constraints.and_then(|c| {
            let mut c = c.clone();
            c.account_for_base_object_size::<Self>();

            Some(c)
        });

        let mut max_size = parent_constraints.as_ref().and_then(|c| c.max_size);

        let mut uninit_struct = std::mem::MaybeUninit::<#name>::uninit();
        let uninit_struct_ptr = uninit_struct.as_mut_ptr();

        if Self::is_variable_size() {
            // this makes for ugly code generation, but better perf
            for i in sample(&mut mutator.rng, #generate_fields_count, #generate_fields_count).iter() {
                match i {
                    #(#generate_arms)*
                    _ => unreachable!(),
                }
            }
        } else {
            #(#generate_linear)*
        }

        let mut initialized_struct = unsafe { uninit_struct.assume_init() };

        if mutator.should_fixup() {
            initialized_struct.fixup(mutator);
        }

        initialized_struct
    }
}
